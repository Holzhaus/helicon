// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use super::ui;
use crate::musicbrainz::{MusicBrainzClient, MusicBrainzRelease};
use crate::release::ReleaseLike;
use crate::release_candidate::{ReleaseCandidate, ReleaseCandidateCollection};
use crate::util::walk_dir;
use crate::Cache;
use crate::{Config, TaggedFile, TaggedFileCollection};
use clap::Parser;
use futures::StreamExt;
use std::collections::HashSet;
use std::path::PathBuf;

/// Command line arguments for the `import` CLI command.
#[derive(Parser, Debug)]
pub struct Args {
    /// Path to import.
    path: PathBuf,
}

/// Find track collections in the given path.
fn find_track_collections(input_path: PathBuf) -> impl Iterator<Item = TaggedFileCollection> {
    let supported_extensions = HashSet::from(["mp3", "flac"]);
    walk_dir(input_path)
        .filter_map(Result::ok)
        .filter_map(move |(path, _dirs, files)| {
            let tagged_files: Vec<TaggedFile> = files
                .iter()
                .filter(|path| {
                    path.extension()
                        .map(std::ffi::OsStr::to_ascii_lowercase)
                        .and_then(|extension| {
                            extension
                                .to_str()
                                .map(|extension| supported_extensions.contains(extension))
                        })
                        .unwrap_or(false)
                })
                .filter_map(|path| match TaggedFile::read_from_path(path) {
                    Ok(file) => Some(file),
                    Err(err) => {
                        log::warn!("Failed to read {}: {}", path.display(), err);
                        None
                    }
                })
                .collect();
            if tagged_files.is_empty() {
                return None;
            }

            log::info!("Found {} tracks in {}", tagged_files.len(), path.display(),);
            Some(TaggedFileCollection::new(tagged_files))
        })
}

/// Result returned from the [`select_release()`] function.
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
    let input_path = args.path;

    let (scanner_tx, mut scanner_rx) = tokio::sync::mpsc::channel(20);
    let cloned_config = config.clone();
    let cloned_cache = cache.cloned();
    let _scanner_handle = tokio::task::spawn(async move {
        let musicbrainz = MusicBrainzClient::new(&cloned_config, cloned_cache.as_ref());
        for track_collection in find_track_collections(input_path) {
            let candidates = match musicbrainz
                .find_releases_by_similarity(&track_collection)
                .await
            {
                Ok(releases) => ReleaseCandidateCollection::new(releases),
                Err(err) => {
                    log::error!("Receiver dropped: {err}");
                    continue;
                }
            };

            let item = (track_collection, candidates);
            if let Err(err) = scanner_tx.send(item).await {
                log::error!("Receiver dropped: {err}");
                continue;
            }
        }
    });

    let (importer_tx, mut importer_rx) = tokio::sync::mpsc::channel::<(
        TaggedFileCollection,
        ReleaseCandidate<MusicBrainzRelease>,
    )>(20);
    let importer_handle = tokio::task::spawn(async move {
        while let Some((track_collection, selected_candidate)) = importer_rx.recv().await {
            let _track_collection = track_collection.assign_tags(&selected_candidate);
        }
    });

    let musicbrainz = MusicBrainzClient::new(config, cache);
    while let Some((track_collection, candidates)) = scanner_rx.recv().await {
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
