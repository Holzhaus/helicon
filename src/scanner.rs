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
use futures::FutureExt;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinSet;

/// An error type that contains the path that was scanned when the error occurred.
pub struct ScanError {
    /// The path for which the error occurred.
    pub path: PathBuf,
    /// The actual error.
    pub source: crate::Error,
}

/// Convenience Alias for a Scan Result.
type ScanResult = Result<
    (
        TaggedFileCollection,
        ReleaseCandidateCollection<MusicBrainzRelease>,
    ),
    ScanError,
>;

/// Scanner struct.
pub struct Scanner {
    /// Worker thread pool.
    ///
    /// This is an option because we want to drop it in the `Drop` impl, but the method only takes
    /// a reference.
    pool: Option<Runtime>,
    /// Channel receiver for the scanner results.
    results_rx: Receiver<ScanResult>,
}

impl Scanner {
    /// Create a scanner for the given path.
    pub fn scan(config: Config, cache: Option<Cache>, path: PathBuf) -> Scanner {
        log::info!("Starting scan of {}", path.display());

        let (results_tx, results_rx) = tokio::sync::mpsc::channel(20);
        let num_parallel_jobs = if config.analyzers.num_parallel_jobs == 0 {
            num_cpus::get()
        } else {
            config.analyzers.num_parallel_jobs
        };
        let pool = Builder::new_multi_thread()
            .max_blocking_threads(num_parallel_jobs)
            .thread_name("scanner-worker")
            .enable_all()
            .build()
            .unwrap();

        let cloned_results_tx = results_tx.clone();
        let pool_handle = pool.handle().clone();
        let _scanner = pool.spawn(async move {
            // First, search the file system to find track paths.
            for (path, tracks) in find_track_paths(path) {
                let cloned_config = config.clone();
                let cloned_config2 = config.clone();

                // Some tracks were found, spawn individual tasks for analyzing the tracks in the
                // threadpool. We keep track of the spawned task handles in a Vec, so that we
                // combine the results of these tasks in a track collection.
                let mut handles = JoinSet::new();
                for track in tracks {
                    let config = cloned_config.clone();
                    let _analysis_abort_handle = handles.spawn_blocking_on(
                        move || analyze_tagged_file(&config, track),
                        &pool_handle,
                    );
                }

                // When all handles are joined, make a collection out of it and search similar
                // releases on MusicBrainz. The result is sent to the `results_tx` queue.
                let cloned_cache = cache.clone();
                let results_tx = cloned_results_tx.clone();
                let _matching_logic = pool_handle.spawn(async move {
                    let musicbrainz =
                        MusicBrainzClient::new(&cloned_config2, cloned_cache.as_ref());
                    if let Err(err) = results_tx
                        .send(
                            join_analysis_tasks_to_collection_and_find_release_candidates(
                                &musicbrainz,
                                path,
                                handles,
                            )
                            .await,
                        )
                        .await
                    {
                        log::error!("Failed to queue results: {err}");
                    }
                });
            }
        });

        Scanner {
            pool: pool.into(),
            results_rx,
        }
    }

    /// Receive the next track collection from the scanner.
    pub async fn recv(&mut self) -> Option<ScanResult> {
        self.results_rx.recv().await
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        if let Some(runtime) = self.pool.take() {
            runtime.shutdown_background();
        }
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

/// Join all analysis tasks, then create a TaggedFieCollection from it. Then find similar
/// candidates on MusicBrainz.
async fn join_analysis_tasks_to_collection_and_find_release_candidates(
    musicbrainz: &MusicBrainzClient<'_>,
    path: PathBuf,
    handles: JoinSet<TaggedFile>,
) -> ScanResult {
    handles
        .join_all()
        .then(|mut tracks| async {
            tracks.sort_unstable_by(|a, b| a.path.as_path().cmp(b.path.as_path()));
            let track_collection = TaggedFileCollection::new(tracks);
            musicbrainz
                .find_releases_by_similarity(&track_collection)
                .await
                .map(ReleaseCandidateCollection::new)
                .map(|candidates| (track_collection, candidates))
        })
        .await
        .map_err(|source| ScanError { path, source })
}
