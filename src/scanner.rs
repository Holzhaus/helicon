// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! The scanner will search a given path for media files, analyze them and find similar releases on
//! [MusicBrainz][mb].
//!
//! [mb]: https://musicbrainz.org

use crate::analyzer;
use crate::musicbrainz::{MusicBrainzClient, MusicBrainzRelease};
use crate::release_candidate::ReleaseCandidateCollection;
use crate::util::walk_dir;
use crate::Cache;
use crate::{Config, TaggedFile, TaggedFileCollection};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::{error::TryRecvError, Receiver};

/// Scanner struct.
pub struct Scanner {
    /// Channel receiver for the scanner results.
    results_rx: Receiver<(
        TaggedFileCollection,
        ReleaseCandidateCollection<MusicBrainzRelease>,
    )>,
}

impl Scanner {
    /// Create a scanner for the given path.
    pub fn scan(config: Config, cache: Option<Cache>, path: PathBuf) -> Scanner {
        log::info!("Starting scan of {}", path.display());

        let (analyzer_input_tx, analyzer_input_rx) = async_channel::bounded(20);
        let (analyzer_group_tx, mut analyzer_group_rx) = tokio::sync::mpsc::channel(5);

        let _fsscanner = tokio::task::spawn(async move {
            let mut group_id: usize = 0;
            for (_path, tracks) in find_track_paths(path) {
                let mut num_tracks: usize = 0;
                for track in tracks {
                    if let Err(err) = analyzer_input_tx.send((group_id, track)).await {
                        log::error!("Receiver dropped on sending track: {err}");
                        continue;
                    }
                    log::debug!("Queued track for group {group_id}");
                    num_tracks += 1;
                }

                if let Err(err) = analyzer_group_tx.send((group_id, num_tracks)).await {
                    log::error!("Receiver dropped on sending path group: {err}");
                } else {
                    log::debug!("Queued group {group_id} with {num_tracks} for analysis");
                };

                group_id += 1;
            }
        });

        let (analyzer_output_tx, mut analyzer_output_rx) = tokio::sync::mpsc::channel(20);
        let num_jobs = Some(config.analyzers.num_parallel_jobs)
            .filter(|&n| n != 0)
            .unwrap_or_else(num_cpus::get);
        for _ in 0..num_jobs {
            let analyzer_input_rx = analyzer_input_rx.clone();
            let analyzer_output_tx = analyzer_output_tx.clone();
            let cloned_config = config.clone();
            let _analysisworker = tokio::task::spawn(async move {
                while let Ok((group_id, track)) = analyzer_input_rx.recv().await {
                    let track = analyze_tagged_file(&cloned_config, track);
                    if let Err(err) = analyzer_output_tx.send((group_id, track)).await {
                        log::error!("Receiver dropped on sending track: {err}");
                    }
                }
            });
        }

        let (post_analysis_tx, mut post_analysis_rx) = tokio::sync::mpsc::channel(20);
        let _analysiscollector = tokio::task::spawn(async move {
            let mut group_track_counts = HashMap::new();
            let mut group_tracks = HashMap::new();

            let mut analyzer_group_rx_connected = true;
            let mut analyzer_output_rx_connected = true;

            #[allow(unused_results)]
            while analyzer_group_rx_connected || analyzer_output_rx_connected {
                while analyzer_group_rx_connected {
                    let (group_id, num_tracks) = match analyzer_group_rx.try_recv() {
                        Ok(result) => result,
                        Err(TryRecvError::Empty) => {
                            break;
                        }
                        Err(TryRecvError::Disconnected) => {
                            analyzer_group_rx_connected = false;
                            break;
                        }
                    };

                    group_track_counts.insert(group_id, num_tracks);
                    group_tracks.insert(group_id, Vec::with_capacity(num_tracks));
                }

                while analyzer_output_rx_connected {
                    let (group_id, track) = match analyzer_output_rx.try_recv() {
                        Ok(result) => result,
                        Err(TryRecvError::Empty) => {
                            break;
                        }
                        Err(TryRecvError::Disconnected) => {
                            analyzer_output_rx_connected = false;
                            break;
                        }
                    };

                    let is_group_finished = if let Some(tracks) = group_tracks.get_mut(&group_id) {
                        tracks.push(track);
                        if let Some(&counts) = group_track_counts.get(&group_id) {
                            tracks.len() >= counts
                        } else {
                            log::error!("Missing track count for group {group_id}");
                            false
                        }
                    } else {
                        false
                    };

                    if !is_group_finished {
                        continue;
                    }

                    group_track_counts.remove(&group_id);
                    let Some(tracks) = group_tracks.remove(&group_id) else {
                        log::error!("Missing tracks for group {group_id}");
                        continue;
                    };

                    let collection = TaggedFileCollection::new(tracks);
                    if let Err(err) = post_analysis_tx.send(collection).await {
                        log::error!("Receiver dropped on sending collection: {err}");
                        continue;
                    }
                }

                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            for (group_id, tracks) in group_tracks.drain() {
                if let Some(&expected_track_count) = group_track_counts.get(&group_id) {
                    if expected_track_count != tracks.len() {
                        log::error!("Missing tracks for group {group_id}");
                    }
                } else {
                    log::error!("Missing track count for group {group_id}");
                }

                let collection = TaggedFileCollection::new(tracks);
                if let Err(err) = post_analysis_tx.send(collection).await {
                    log::error!("Receiver dropped on sending collection: {err}");
                }
            }
        });

        let (results_tx, results_rx) = tokio::sync::mpsc::channel(20);
        let _matcher = tokio::task::spawn(async move {
            let config = config;
            let musicbrainz = MusicBrainzClient::new(&config, cache.as_ref());
            while let Some(track_collection) = post_analysis_rx.recv().await {
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
                if let Err(err) = results_tx.send(item).await {
                    log::error!("Receiver dropped: {err}");
                    continue;
                }
            }
        });

        Scanner { results_rx }
    }

    /// Receive the next track collection from the scanner.
    pub async fn recv(
        &mut self,
    ) -> Option<(
        TaggedFileCollection,
        ReleaseCandidateCollection<MusicBrainzRelease>,
    )> {
        self.results_rx.recv().await
    }
}

/// Find track collections in the given path.
fn find_track_paths(input_path: PathBuf) -> impl Iterator<Item = (PathBuf, Vec<TaggedFile>)> {
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

            log::info!("Found {} tracks in {}", tagged_files.len(), path.display());

            Some((path, tagged_files))
        })
}

/// Analyze a file and assign the analysis results to it.
fn analyze_tagged_file(config: &Config, tagged_file: TaggedFile) -> TaggedFile {
    let path = tagged_file.path.as_path();
    let analysis_result = analyzer::analyze(config, path)
        .inspect_err(|err| {
            log::warn!("Analysis of {path} failed: {err}", path = path.display());
        })
        .ok();
    tagged_file.with_analysis_results(analysis_result)
}
