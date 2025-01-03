// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use super::ui;
use crate::media::MediaLike;
use crate::musicbrainz::{MusicBrainzClient, MusicBrainzRelease};
use crate::release::ReleaseLike;
use crate::release_candidate::{ReleaseCandidate, ReleaseCandidateCollection};
use crate::scanner::Scanner;
use crate::track::TrackLike;
use crate::util::FormattedDuration;
use crate::Cache;
use crate::{Config, TaggedFileCollection};
use clap::Parser;
use futures::StreamExt;
use std::borrow::Cow;
use std::path::PathBuf;

/// Command line arguments for the `import` CLI command.
#[derive(Parser, Debug)]
pub struct Args {
    /// Path to import.
    path: PathBuf,
}

/// Result returned from the [`select_release()`] function.
#[allow(clippy::large_enum_variant)]
enum SelectionResult {
    /// A candidate was selected and should be assigned to the track collection.
    Selected(TaggedFileCollection, ReleaseCandidate<MusicBrainzRelease>),
    /// Skip importing the track collection.
    Skipped,
    /// Quit import.
    Quit,
}

/// Select the release for the given track collection from the list of candidates.
async fn select_release<'a>(
    config: &Config,
    musicbrainz: &'a MusicBrainzClient<'a>,
    track_collection: TaggedFileCollection,
    mut candidates: ReleaseCandidateCollection<MusicBrainzRelease>,
) -> crate::Result<SelectionResult> {
    println!(
        "Tagging: {artist} - {title} ({track_count} tracks)",
        artist = track_collection
            .release_artist()
            .unwrap_or("[unknown artist]".into()),
        title = track_collection
            .release_title()
            .unwrap_or("[unknown title]".into()),
        track_count = track_collection.release_track_count().unwrap_or(0),
    );
    let mut allow_autoselection = candidates.len() == 1;
    'select_candidate: loop {
        let selected_candidate: &ReleaseCandidate<_> = loop {
            match ui::select_candidate(config, &candidates, allow_autoselection)? {
                ui::ReleaseCandidateSelectionResult::Candidate(candidate) => break candidate,
                ui::ReleaseCandidateSelectionResult::FetchCandidateRelease(release_id) => {
                    log::debug!("Manually adding release candidate with release ID {release_id}");
                    let release = musicbrainz.find_release_by_id(release_id).await?;
                    candidates.add_release(release, &track_collection, config);
                }
                ui::ReleaseCandidateSelectionResult::FetchCandidateReleaseGroup(
                    release_group_id,
                ) => {
                    log::debug!("Manually adding release candidate with release group ID {release_group_id}");
                    candidates = musicbrainz
                        .find_releases_by_release_group_id(release_group_id)
                        .await?
                        .fold(candidates, |mut acc, result| async {
                            let release = match result {
                                Ok(release) => release,
                                Err(err) => {
                                    log::warn!("Failed to retrieve release: {err}");
                                    return acc;
                                }
                            };

                            acc.add_release(release, &track_collection, config);
                            acc
                        })
                        .await;
                }
                ui::ReleaseCandidateSelectionResult::PrintTrackList => {
                    print_tracklist(&track_collection);
                    continue 'select_candidate;
                }
                #[cfg(feature = "dev")]
                ui::ReleaseCandidateSelectionResult::DumpReleaseInfo => {
                    use crate::util::FakeRelease;
                    use std::fs::File;

                    let tmp_dir = tempfile::Builder::new()
                        .prefix("helicon-release-info-dump-")
                        .tempdir()?;
                    let mut saved_anything = false;
                    'save_debuginfo: for (path, release) in std::iter::once((
                        tmp_dir.path().join("release.json"),
                        FakeRelease::from(&track_collection),
                    ))
                    .chain(candidates.iter().enumerate().map(|(i, candidate)| {
                        (
                            tmp_dir.path().join(format!("candidate_{i}.json")),
                            FakeRelease::from(candidate.release()),
                        )
                    })) {
                        let file = match File::create(path.as_path()) {
                            Ok(f) => f,
                            Err(err) => {
                                eprintln!(
                                    "Failed to open {path} for writing: {err}",
                                    path = path.display()
                                );
                                continue 'save_debuginfo;
                            }
                        };

                        if let Err(err) = serde_json::to_writer_pretty(file, &release) {
                            eprintln!(
                                "Failed to write debug info to {path}: {err}",
                                path = path.display()
                            );
                            continue 'save_debuginfo;
                        }
                        saved_anything = true;
                    }

                    if saved_anything {
                        eprintln!("Saved debug info to {}", tmp_dir.into_path().display());
                    } else {
                        eprintln!("Nothing to do");
                    }

                    continue 'select_candidate;
                }
                ui::ReleaseCandidateSelectionResult::Skipped => {
                    return Ok(SelectionResult::Skipped)
                }
                ui::ReleaseCandidateSelectionResult::Quit => return Ok(SelectionResult::Quit),
            };
        };
        allow_autoselection = false;

        match ui::handle_candidate(config, &track_collection, selected_candidate)? {
            ui::HandleCandidateResult::Apply => {
                let candidate_index = candidates.find_index(selected_candidate);
                let selected_candidate = candidates.select_index(candidate_index);
                return Ok(SelectionResult::Selected(
                    track_collection,
                    selected_candidate,
                ));
            }
            ui::HandleCandidateResult::Skip => {
                log::warn!("Skipping collection");
                return Ok(SelectionResult::Skipped);
            }
            ui::HandleCandidateResult::BackToSelection => {
                continue 'select_candidate;
            }
            ui::HandleCandidateResult::Quit => {
                return Ok(SelectionResult::Quit);
            }
            ui::HandleCandidateResult::ShowDetails
            | ui::HandleCandidateResult::HideDetails
            | ui::HandleCandidateResult::PrintTrackList => {
                unreachable!()
            }
        }
    }
}

/// Print Tracklist (for copy & paste into MusicBrainz).
fn print_tracklist(release: &impl ReleaseLike) {
    for (media_index, media) in release.media().enumerate() {
        let format = media.media_format().unwrap_or_else(|| "Medium".into());
        if media_index > 0 {
            println!();
        }
        if let Some(title) = media.media_title() {
            println!("{format} {index}: {title}", index = media_index + 1);
        } else {
            println!("{format} {index}", index = media_index + 1);
        }
        for (track_index, track) in media.media_tracks().enumerate() {
            let track_number = track
                .track_number()
                .unwrap_or_else(|| Cow::from(format!("{track_index}")));
            let track_artist = track
                .track_artist()
                .unwrap_or_else(|| Cow::from("[unknown]"));
            let track_title = track
                .track_title()
                .unwrap_or_else(|| Cow::from("[unnamed track]"));
            let track_length = track.track_length().map_or_else(
                || Cow::from("?:??"),
                |length| Cow::from(length.formatted_duration()),
            );
            println!("{track_number}. {track_title} - {track_artist} ({track_length})");
        }
    }
}

/// Run an import.
///
/// # Errors
///
/// If the underlying [`walk_dir`] function encounters any form of I/O or other error, an error
/// variant will be returned.
pub async fn run(config: &Config, cache: Option<&Cache>, args: Args) -> crate::Result<()> {
    let mut scanner = Scanner::scan(config.clone(), cache.cloned(), args.path);

    let (importer_tx, mut importer_rx) = tokio::sync::mpsc::channel::<(
        TaggedFileCollection,
        ReleaseCandidate<MusicBrainzRelease>,
    )>(20);
    let cloned_config = config.clone();
    let importer_handle = tokio::task::spawn(async move {
        while let Some((track_collection, selected_candidate)) = importer_rx.recv().await {
            let mut track_collection = track_collection.assign_tags(&selected_candidate);
            if let Err(err) = track_collection.move_files(&cloned_config) {
                log::error!("Failed to move files: {err}");
                continue;
            };

            if let Err(err) = track_collection.write_tags() {
                log::error!("Failed to write tags: {err}");
            };
        }
    });

    let musicbrainz = MusicBrainzClient::new(config, cache);
    while let Some(result) = scanner.recv().await {
        let (track_collection, candidates) = match result {
            Ok(res) => res,
            Err(err) => {
                log::error!("Scan of {} failed: {}", err.path.display(), err.source);
                continue;
            }
        };
        match select_release(config, &musicbrainz, track_collection, candidates).await? {
            SelectionResult::Selected(track_collection, selected_candidate) => {
                if let Err(err) = importer_tx
                    .send((track_collection, selected_candidate))
                    .await
                {
                    log::error!("Failed to send job to importer: {err}");
                };
            }
            SelectionResult::Skipped => {
                continue;
            }
            SelectionResult::Quit => {
                break;
            }
        };
    }

    drop(importer_tx);
    importer_handle.await.unwrap();

    Ok(())
}
