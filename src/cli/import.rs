// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use crate::distance::DistanceItem;
use crate::musicbrainz;
use crate::release::ReleaseLike;
use crate::util::walk_dir;
use crate::{Config, TaggedFile, TaggedFileCollection};
use clap::Parser;
use futures::{future, stream::StreamExt};
use std::collections::{BinaryHeap, HashSet};
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
pub async fn run(config: &Config, args: Args) -> crate::Result<()> {
    let input_path = args.path;

    let supported_extensions = HashSet::from(["mp3", "flac"]);
    for item in walk_dir(input_path) {
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

        let mut heap = BinaryHeap::new();
        musicbrainz::find_releases(&track_collection)
            .filter_map(|result| async {
                match result {
                    Ok(release) => Some(release),
                    Err(err) => {
                        log::warn!("Failed to retrieve release: {}", err);
                        None
                    }
                }
            })
            .for_each(|release| {
                let release_similarity = track_collection.similarity_to(&release, config);
                let release_distance = release_similarity.total_distance();
                log::debug!(
                    "Release '{}' has distance to track collection: {}",
                    release.title,
                    release_distance.weighted_distance()
                );
                let item = DistanceItem::new((release, release_similarity), release_distance);
                heap.push(item);
                future::ready(())
            })
            .await;

        log::info!("Found {} release candidates.", heap.len());
        heap.into_sorted_vec()
            .iter()
            .enumerate()
            .for_each(|(index, candidate)| {
                log::info!(
                    "{:02}. {} - {} ({}distance: {:.3})",
                    index + 1,
                    candidate.item.0.release_artist().unwrap_or_default(),
                    candidate.item.0.release_title().unwrap_or_default(),
                    candidate
                        .item
                        .0
                        .track_count()
                        .map(|c| format!("{c} tracks, "))
                        .unwrap_or_default(),
                    candidate.distance().weighted_distance()
                );
            });
    }

    Ok(())
}
