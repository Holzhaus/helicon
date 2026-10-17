// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use super::ui;
use crate::coverart::CoverInfo;
use crate::media::MediaLike;
use crate::musicbrainz::{MusicBrainzClient, MusicBrainzRelease};
use crate::release::ReleaseLike;
use crate::release_candidate::{ReleaseCandidate, ReleaseCandidateCollection};
use crate::scanner::Scanner;
use crate::taggedfilecollection::TransferMode;
use crate::track::TrackLike;
use crate::util::FormattedDuration;
use crate::{Config, TaggedFileCollection};
use clap::Parser;
use crossterm::style::Stylize;
use futures::StreamExt;
use std::borrow::Cow;
use std::path::PathBuf;

/// Command line arguments for the `import` CLI command.
#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools)]
pub struct Args {
    /// Path to import.
    path: PathBuf,

    /// Copy tracks into library directory (default).
    #[arg(short = 'c', long, overrides_with = "nocopy")]
    copy: bool,

    /// Don't copy tracks (opposite of -c).
    #[arg(short = 'C', long, overrides_with = "copy")]
    nocopy: bool,

    /// Move tracks into the library (overrides -c).
    #[arg(short = 'm', long = "move", id = "move", overrides_with = "nomove")]
    r#move: bool,

    /// Don't move tracks into the library (overrides -m).
    #[arg(short = 'M', long, overrides_with = "move")]
    nomove: bool,
}

impl Args {
    /// Determine how files should be transferred into the library.
    fn transfer_mode(&self) -> TransferMode {
        if self.r#move {
            TransferMode::Move
        } else if !self.nocopy {
            TransferMode::Copy
        } else {
            TransferMode::InPlace
        }
    }
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
async fn select_release(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    track_collection: TaggedFileCollection,
    mut candidates: ReleaseCandidateCollection<MusicBrainzRelease>,
) -> crate::Result<SelectionResult> {
    println!(
        "Tagging: {release} ({track_count} tracks)",
        release = format!(
            "{artist} - {title}",
            artist = track_collection
                .release_artist()
                .unwrap_or("[unknown artist]".into()),
            title = track_collection
                .release_title()
                .unwrap_or("[unknown album]".into()),
        )
        .bold(),
        track_count = track_collection.release_track_count().unwrap_or(0),
    );
    if let Some(directory) = track_collection.common_directory() {
        println!(
            "{}",
            config
                .user_interface
                .candidate_details
                .release_meta_style
                .apply(directory.display())
        );
    }
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
                        eprintln!("Saved debug info to {}", tmp_dir.keep().display());
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

        let cover_art =
            build_cover_art_preview(config, musicbrainz, &track_collection, selected_candidate)
                .await;
        match ui::handle_candidate(
            config,
            &track_collection,
            selected_candidate,
            cover_art.as_ref(),
        )? {
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
            ui::HandleCandidateResult::BackToSelection => (),
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

/// Fetch the front cover of the candidate to preview what will be embedded.
async fn build_cover_art_preview(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    track_collection: &TaggedFileCollection,
    candidate: &ReleaseCandidate<MusicBrainzRelease>,
) -> Option<ui::CoverArtPreview> {
    let cover_art_config = config.cover_art;
    if !cover_art_config.enabled {
        return None;
    }

    let new_cover = match musicbrainz
        .find_front_cover_by_release_id(&candidate.release().id)
        .await
    {
        Ok(cover) => cover.map(|cover| cover.description(cover_art_config.resolution)),
        Err(err) => {
            log::warn!("Failed to fetch front cover art: {err}");
            None
        }
    };
    Some(ui::CoverArtPreview {
        existing_covers: track_collection
            .front_cover_infos()
            .iter()
            .map(|info| info.as_ref().map(CoverInfo::description))
            .collect(),
        new_cover,
        overwrite_existing: cover_art_config.overwrite_existing,
    })
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
                .or_else(|| track.track_file_stem())
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
pub async fn run(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    args: Args,
) -> crate::Result<()> {
    let transfer_mode = args.transfer_mode();
    let mut scanner = Scanner::scan(config.clone(), musicbrainz.clone(), args.path);

    let (importer_tx, mut importer_rx) = tokio::sync::mpsc::channel::<(
        TaggedFileCollection,
        ReleaseCandidate<MusicBrainzRelease>,
    )>(20);
    let cloned_config = config.clone();
    let cloned_musicbrainz = musicbrainz.clone();
    let importer_handle = tokio::task::spawn(async move {
        while let Some((track_collection, selected_candidate)) = importer_rx.recv().await {
            let mut track_collection = track_collection.assign_tags(&selected_candidate);
            if let Err(err) = track_collection.transfer_files(&cloned_config, transfer_mode) {
                log::error!("Failed to transfer files: {err}");
                continue;
            }

            let cover_art_config = cloned_config.cover_art;
            if cover_art_config.enabled
                && track_collection.needs_front_cover(cover_art_config.overwrite_existing)
            {
                match cloned_musicbrainz
                    .find_front_cover_by_release_id(&selected_candidate.release().id)
                    .await
                {
                    Ok(Some(cover)) => {
                        log::info!(
                            "Embedding front cover art ({}, {} bytes)",
                            cover.mime_type(),
                            cover.data().len()
                        );
                        track_collection
                            .set_front_cover(&cover, cover_art_config.overwrite_existing);
                    }
                    Ok(None) => log::info!("No front cover art available for this release"),
                    Err(err) => log::warn!("Failed to fetch front cover art: {err}"),
                }
            }

            if let Err(err) = track_collection.write_tags() {
                log::error!("Failed to write tags: {err}");
            }

            #[cfg(unix)]
            if let Err(err) = track_collection.set_permissions(&cloned_config) {
                log::error!("Failed to set permissions: {err}");
            }
        }
    });

    while let Some(result) = scanner.recv().await {
        let (track_collection, candidates) = match result {
            Ok(res) => res,
            Err(err) => {
                log::error!("Scan of {} failed: {}", err.path.display(), err.source);
                continue;
            }
        };
        match select_release(config, musicbrainz, track_collection, candidates).await? {
            SelectionResult::Selected(track_collection, selected_candidate) => {
                if let Err(err) = importer_tx
                    .send((track_collection, selected_candidate))
                    .await
                {
                    log::error!("Failed to send job to importer: {err}");
                };
            }
            SelectionResult::Skipped => (),
            SelectionResult::Quit => {
                break;
            }
        };
    }

    drop(importer_tx);
    importer_handle.await.unwrap();

    Ok(())
}
