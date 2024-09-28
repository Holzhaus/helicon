// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use super::ui;
use crate::musicbrainz::MusicBrainzClient;
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

/// Run an import.
///
/// # Errors
///
/// If the underlying [`walk_dir`] function encounters any form of I/O or other error, an error
/// variant will be returned.
pub async fn run(config: &Config, cache: Option<&Cache>, args: Args) -> crate::Result<()> {
    let input_path = args.path;

    let (tx, mut rx) = tokio::sync::mpsc::channel(20);
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
            if let Err(err) = tx.send(item).await {
                log::error!("Receiver dropped: {err}");
                continue;
            }
        }
    });

    let musicbrainz = MusicBrainzClient::new(config, cache);
    'handle_next_collection: while let Some((track_collection, mut candidates)) = rx.recv().await {
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
                match ui::select_candidate(config, &candidates, allow_autoselection) {
                    Ok(ui::ReleaseCandidateSelectionResult::Candidate(candidate)) => {
                        break candidate
                    }
                    Ok(ui::ReleaseCandidateSelectionResult::FetchCandidateRelease(release_id)) => {
                        let release = musicbrainz.find_release_by_id(release_id).await?;
                        candidates.add_release(release, &track_collection, config);
                    }
                    Ok(ui::ReleaseCandidateSelectionResult::FetchCandidateReleaseGroup(
                        release_group_id,
                    )) => {
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
                    Err(inquire::InquireError::OperationInterrupted) => {
                        Err(inquire::InquireError::OperationInterrupted)?;
                    }
                    Err(err) => {
                        log::warn!("Selection failed: {err}");
                        continue 'handle_next_collection;
                    }
                };
            };
            allow_autoselection = false;

            match ui::handle_candidate(config, &track_collection, selected_candidate)? {
                ui::HandleCandidateResult::Apply => todo!(),
                ui::HandleCandidateResult::Skip => {
                    log::warn!("Skipping collection");
                    continue 'handle_next_collection;
                }
                ui::HandleCandidateResult::BackToSelection => {
                    continue 'select_candidate;
                }
            }
        }
    }

    Ok(())
}
