// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use super::ui;
use crate::musicbrainz;
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

/// Run an import.
///
/// # Errors
///
/// If the underlying [`walk_dir`] function encounters any form of I/O or other error, an error
/// variant will be returned.
pub async fn run(config: &Config, cache: Option<&impl Cache>, args: Args) -> crate::Result<()> {
    let input_path = args.path;

    let supported_extensions = HashSet::from(["mp3", "flac"]);
    'handle_next_collection: for item in walk_dir(input_path) {
        let (path, _dirs, files) = item?;
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
            continue;
        }

        log::info!(
            "Tagging: {} ({} tracks)",
            path.display(),
            tagged_files.len()
        );

        let track_collection = TaggedFileCollection::new(tagged_files);

        let mut candidates = ReleaseCandidateCollection::new(
            musicbrainz::find_releases(config, cache, &track_collection).await?,
        );
        'select_candidate: loop {
            let selected_candidate: &ReleaseCandidate<_> = loop {
                match ui::select_candidate(&candidates) {
                    Ok(ui::ReleaseCandidateSelectionResult::Candidate(candidate)) => {
                        break candidate
                    }
                    Ok(ui::ReleaseCandidateSelectionResult::FetchCandidateRelease(mb_id)) => {
                        let release = musicbrainz::find_release_by_mb_id(mb_id, cache).await?;
                        candidates.add_release(release, &track_collection, config);
                    }
                    Ok(ui::ReleaseCandidateSelectionResult::FetchCandidateReleaseGroup(mb_id)) => {
                        candidates =
                            musicbrainz::find_releases_by_release_group_id(config, cache, mb_id)
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
                    Err(err) => {
                        log::warn!("Selection failed: {err}");
                        continue 'handle_next_collection;
                    }
                };
            };

            match ui::handle_candidate(selected_candidate)? {
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
