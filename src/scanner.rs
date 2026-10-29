// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
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
use crate::release::ReleaseLike;
use crate::release_candidate::{ReleaseCandidate, ReleaseCandidateCollection};
use crate::tag::is_supported_extension;
use crate::util::{walk_dir, DirWalk};
use crate::{Config, TaggedFile, TaggedFileCollection};
use regex::{Regex, RegexBuilder};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::{Builder, Handle, Runtime};
use tokio::sync::mpsc::Receiver;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;

/// An error type that contains the path that was scanned when the error occurred.
pub struct ScanError {
    /// The path for which the error occurred.
    pub path: PathBuf,
    /// The actual error.
    pub source: Box<crate::Error>,
}

/// Convenience Alias for a Scan Result.
type ScanResult = Result<
    (
        TaggedFileCollection,
        ReleaseCandidateCollection<MusicBrainzRelease>,
    ),
    ScanError,
>;

/// Maximum number of track collections that are being analyzed, matched or waiting to be sent
/// to the consumer at the same time. Discovery pauses while this limit is reached.
const MAX_COLLECTIONS_IN_FLIGHT: usize = 10;

/// Capacity of the channel that delivers finished results to the consumer.
const RESULTS_CHANNEL_CAPACITY: usize = 20;

/// Scanner struct.
///
/// Dropping the scanner cancels the scan: discovery stops, queued analysis jobs are discarded
/// and pending MusicBrainz requests are dropped. Analysis jobs that are already running cannot be
/// interrupted and finish in the background.
pub struct Scanner {
    /// Worker thread pool.
    ///
    /// This is an option because we want to drop it in the `Drop` impl, but the method only takes
    /// a reference.
    pool: Option<Runtime>,
    /// Channel receiver for the scanner results.
    results_rx: Receiver<ScanResult>,
    /// Limits the number of collections in flight. Closed on drop to stop discovery.
    limiter: Arc<Semaphore>,
}

impl Scanner {
    /// Create a scanner for the given path.
    ///
    /// If `refresh_only` is set, only collections with a consensual MusicBrainz release ID are
    /// considered. The release with that ID is the only candidate; no similarity search is done.
    pub fn scan(
        config: Config,
        musicbrainz: MusicBrainzClient,
        path: PathBuf,
        refresh_only: bool,
    ) -> Scanner {
        log::info!("Starting scan of {}", path.display());

        let (results_tx, results_rx) = tokio::sync::mpsc::channel(RESULTS_CHANNEL_CAPACITY);
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

        let pool_handle = pool.handle().clone();
        let limiter = Arc::new(Semaphore::new(MAX_COLLECTIONS_IN_FLIGHT));
        let discovery_limiter = Arc::clone(&limiter);
        // Discovery does blocking file I/O and must not occupy the (bounded) analysis threads or
        // the async workers, so it gets a dedicated thread. Groups are processed as soon as they
        // are complete, so analysis starts while the file system is still being walked.
        let spawn_result = std::thread::Builder::new()
            .name("scanner-discovery".into())
            .spawn(move || {
                let analysis_handle = pool_handle.clone();
                dispatch_groups(
                    find_track_paths(path),
                    &discovery_limiter,
                    &pool_handle,
                    |(path, tracks), permit| {
                        // Spawn individual tasks for analyzing the tracks in the threadpool. We
                        // keep track of the spawned task handles, so that we can combine the
                        // results of these tasks in a track collection.
                        let mut handles = JoinSet::new();
                        for track in tracks {
                            let config = config.clone();
                            let _analysis_abort_handle = handles.spawn_blocking_on(
                                move || analyze_tagged_file(&config, track),
                                &analysis_handle,
                            );
                        }

                        // When all handles are joined, make a collection out of it and search
                        // similar releases on MusicBrainz. The result is sent to the `results_tx`
                        // queue. The permit is held until the result was accepted by the queue,
                        // so a slow consumer eventually pauses discovery.
                        let config = config.clone();
                        let musicbrainz = musicbrainz.clone();
                        let results_tx = results_tx.clone();
                        let _matching_logic = analysis_handle.spawn(async move {
                            let _permit = permit;
                            let Some(result) =
                                join_analysis_tasks_to_collection_and_find_release_candidates(
                                    &config,
                                    &musicbrainz,
                                    path,
                                    handles,
                                    refresh_only,
                                )
                                .await
                            else {
                                return;
                            };
                            if let Err(err) = results_tx.send(result).await {
                                log::error!("Failed to queue results: {err}");
                            }
                        });
                    },
                );
            });
        if let Err(err) = spawn_result {
            log::error!("Failed to start discovery thread: {err}");
        }

        Scanner {
            pool: pool.into(),
            results_rx,
            limiter,
        }
    }

    /// Receive the next track collection from the scanner.
    pub async fn recv(&mut self) -> Option<ScanResult> {
        self.results_rx.recv().await
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        // Wakes up and stops the discovery thread, even if it is waiting for a free slot.
        self.limiter.close();
        if let Some(runtime) = self.pool.take() {
            runtime.shutdown_background();
        }
    }
}

/// Feed `groups` to `dispatch`, but never have more than the limiter's permits in flight.
///
/// A permit is acquired *before* the next group is pulled from the iterator, so no work (such as
/// reading tags) is done ahead while the limit is reached. The permit is handed to `dispatch` and
/// released when dropped. Stops once the limiter is closed.
fn dispatch_groups<T>(
    mut groups: impl Iterator<Item = T>,
    limiter: &Arc<Semaphore>,
    handle: &Handle,
    mut dispatch: impl FnMut(T, OwnedSemaphorePermit),
) {
    loop {
        let Ok(permit) = handle.block_on(Arc::clone(limiter).acquire_owned()) else {
            log::debug!("Scanner was shut down, aborting discovery");
            return;
        };
        let Some(group) = groups.next() else {
            return;
        };
        dispatch(group, permit);
    }
}

/// Matches names of directories that contain a single disc of a release, e.g. `CD1`, `Disc 02`,
/// `disk_3` or `Disc 1 - Title`. A title must be separated by punctuation, so that names like
/// `CD1 Bonus` are not mistaken for disc directories.
const DISC_DIRECTORY_PATTERN: &str = r"^(?:CD|Disc|Disk)[\s._-]*\d+(?:\s*[-–:._]\s*\S.*)?$";

/// Lazily yields track collections found below a path.
///
/// The directory walk is a depth-first traversal, so the contents of a directory are visited
/// consecutively. A collection (which may span a directory and its `CD1`/`Disc 2` subdirectories)
/// is therefore complete as soon as the walk moves to a directory that is not inside it, and is
/// yielded at that point. Tags are only read for directories the iterator has advanced to.
struct TrackGroups {
    /// The directory the scan started in. Collections never extend above it.
    root: PathBuf,
    /// Directory walker.
    walk: DirWalk,
    /// Matches names of directories that contain a single disc of a release.
    disc_pattern: Regex,
    /// Collections that may still receive tracks from directories that are yet to be visited.
    pending: BTreeMap<PathBuf, Vec<TaggedFile>>,
    /// Complete collections that were not yielded yet.
    ready: VecDeque<(PathBuf, Vec<TaggedFile>)>,
}

impl TrackGroups {
    /// Create a lazy collection finder for the given path.
    fn new(input_path: PathBuf) -> Self {
        Self {
            walk: walk_dir(input_path.clone()),
            root: input_path,
            disc_pattern: RegexBuilder::new(DISC_DIRECTORY_PATTERN)
                .case_insensitive(true)
                .build()
                .unwrap(),
            pending: BTreeMap::new(),
            ready: VecDeque::new(),
        }
    }

    /// Returns the path of the collection that tracks in the given directory belong to.
    ///
    /// Disc directories belong to their parent directory, unless that would leave the scanned
    /// directory.
    fn group_path(&self, dir: &Path) -> PathBuf {
        let is_disc_dir = dir
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|name| self.disc_pattern.is_match(name));
        match dir.parent() {
            Some(parent) if is_disc_dir && parent.starts_with(&self.root) => parent.to_path_buf(),
            _ => dir.to_path_buf(),
        }
    }

    /// Move all pending collections that can not contain `dir` or its descendants to `ready`.
    fn flush_finished(&mut self, dir: &Path) {
        let finished: Vec<PathBuf> = self
            .pending
            .keys()
            .filter(|group_path| !dir.starts_with(group_path))
            .cloned()
            .collect();
        for group_path in finished {
            if let Some(tracks) = self.pending.remove(&group_path) {
                self.ready.push_back((group_path, tracks));
            }
        }
    }

    /// Read the supported files of a visited directory and add them to their collection.
    fn visit(&mut self, dir: &Path, files: &[PathBuf]) {
        self.flush_finished(dir);

        let tagged_files: Vec<TaggedFile> = files
            .iter()
            .filter(|path| {
                path.extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .is_some_and(is_supported_extension)
            })
            .filter_map(|path| match TaggedFile::read_from_path(path) {
                Ok(file) => Some(file),
                Err(err) => {
                    log::warn!("Failed to read {}: {:?}", path.display(), err);
                    None
                }
            })
            .collect();

        if tagged_files.is_empty() {
            return;
        }

        log::info!("Found {} tracks in {}", tagged_files.len(), dir.display());
        let group_path = self.group_path(dir);
        self.pending
            .entry(group_path)
            .or_default()
            .extend(tagged_files);
    }
}

impl Iterator for TrackGroups {
    type Item = (PathBuf, Vec<TaggedFile>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(group) = self.ready.pop_front() {
                return Some(group);
            }

            match self.walk.next() {
                Some(Ok((dir, _dirs, files))) => self.visit(&dir, &files),
                Some(Err(err)) => log::warn!("Failed to read directory: {err}"),
                None => {
                    // Walk finished, everything that is left is complete.
                    let pending = std::mem::take(&mut self.pending);
                    self.ready.extend(pending);
                    return self.ready.pop_front();
                }
            }
        }
    }
}

/// Find track collections in the given path.
///
/// Collections are yielded lazily, while the file system is being walked.
fn find_track_paths(input_path: PathBuf) -> impl Iterator<Item = (PathBuf, Vec<TaggedFile>)> {
    TrackGroups::new(input_path)
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
///
/// In refresh-only mode, the release referenced by the tags is the only candidate. Returns `None`
/// if the collection should be skipped.
async fn join_analysis_tasks_to_collection_and_find_release_candidates(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    path: PathBuf,
    handles: JoinSet<TaggedFile>,
    refresh_only: bool,
) -> Option<ScanResult> {
    let mut tracks = handles.join_all().await;
    tracks.sort_unstable_by(|a, b| a.path.as_path().cmp(b.path.as_path()));
    let track_collection = TaggedFileCollection::new(tracks);

    let candidates = if refresh_only {
        let Some(release_id) = track_collection
            .musicbrainz_release_id()
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty())
        else {
            log::info!(
                "Skipping {}: no consensual MusicBrainz release ID",
                path.display()
            );
            return None;
        };
        musicbrainz
            .find_release_by_id(release_id)
            .await
            .map(|release| {
                let candidate =
                    ReleaseCandidate::with_base_release(release, &track_collection, config);
                ReleaseCandidateCollection::from(vec![candidate])
            })
    } else {
        musicbrainz
            .find_releases_by_similarity(&track_collection)
            .await
            .map(ReleaseCandidateCollection::from)
    };

    Some(
        candidates
            .map(|candidates| (track_collection, candidates))
            .map_err(|source| ScanError {
                path,
                source: Box::new(source),
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::Path;

    const FLAC_FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/media/untagged/track.flac"
    );

    /// Groups as `relative directory -> sorted relative file names`, independent of iteration
    /// order, so the tests keep working if discovery is made streaming or deterministic.
    type Groups = BTreeMap<PathBuf, Vec<PathBuf>>;

    /// Create a file at `relative` (below `root`), copying `source` or writing garbage if `None`.
    fn add_file(root: &Path, relative: &str, source: Option<&str>) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        match source {
            Some(source) => {
                let _ = std::fs::copy(source, &path).unwrap();
            }
            None => std::fs::write(&path, b"not a media file").unwrap(),
        }
    }

    fn scan_groups(root: &Path) -> Groups {
        let mut groups = Groups::new();
        for (path, tracks) in find_track_paths(root.to_path_buf()) {
            let relative_dir = path.strip_prefix(root).unwrap().to_path_buf();
            let mut files: Vec<PathBuf> = tracks
                .iter()
                .map(|track| track.path.strip_prefix(root).unwrap().to_path_buf())
                .collect();
            files.sort();
            assert!(
                groups.insert(relative_dir, files).is_none(),
                "group {} was yielded twice",
                path.display()
            );
        }
        groups
    }

    fn group(entries: &[(&str, &[&str])]) -> Groups {
        entries
            .iter()
            .map(|(dir, files)| {
                (
                    PathBuf::from(dir),
                    files.iter().map(PathBuf::from).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn empty_directory_yields_no_groups() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_groups(dir.path()).is_empty());
    }

    #[cfg(feature = "flac")]
    #[test]
    fn groups_tracks_by_directory() {
        let dir = tempfile::tempdir().unwrap();
        add_file(dir.path(), "A/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "A/02.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "B/01.flac", Some(FLAC_FIXTURE));

        assert_eq!(
            scan_groups(dir.path()),
            group(&[("A", &["A/01.flac", "A/02.flac"]), ("B", &["B/01.flac"])])
        );
    }

    #[cfg(feature = "flac")]
    #[test]
    fn tracks_in_scan_root_are_grouped_under_root() {
        let dir = tempfile::tempdir().unwrap();
        add_file(dir.path(), "01.flac", Some(FLAC_FIXTURE));

        assert_eq!(scan_groups(dir.path()), group(&[("", &["01.flac"])]));
    }

    #[cfg(feature = "flac")]
    #[test]
    fn ignores_unsupported_and_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        add_file(dir.path(), "A/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "A/cover.jpg", None);
        add_file(dir.path(), "A/notes.txt", None);
        // Supported extension, but not parseable.
        add_file(dir.path(), "A/broken.flac", None);
        // A directory with only unusable files produces no group at all.
        add_file(dir.path(), "B/cover.jpg", None);

        assert_eq!(scan_groups(dir.path()), group(&[("A", &["A/01.flac"])]));
    }

    #[cfg(feature = "flac")]
    #[test]
    fn extension_match_is_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        add_file(dir.path(), "A/01.FLAC", Some(FLAC_FIXTURE));

        assert_eq!(scan_groups(dir.path()), group(&[("A", &["A/01.FLAC"])]));
    }

    #[cfg(feature = "flac")]
    #[test]
    fn disc_directories_are_merged_into_parent() {
        let dir = tempfile::tempdir().unwrap();
        add_file(dir.path(), "Album/CD1/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/CD1/02.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/cd2/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/Disc 3/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/disc4/01.flac", Some(FLAC_FIXTURE));

        assert_eq!(
            scan_groups(dir.path()),
            group(&[(
                "Album",
                &[
                    "Album/CD1/01.flac",
                    "Album/CD1/02.flac",
                    "Album/Disc 3/01.flac",
                    "Album/cd2/01.flac",
                    "Album/disc4/01.flac",
                ]
            )])
        );
    }

    #[cfg(feature = "flac")]
    #[test]
    fn disc_directory_name_variants_are_merged() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "CD01",
            "CD 2",
            "cd_3",
            "Disk 4",
            "Disc-5",
            "Disc 6 - Bonus Disc",
            "DISC7: Live",
            "Disc 8. Remixes",
        ] {
            add_file(
                dir.path(),
                &format!("Album/{name}/01.flac"),
                Some(FLAC_FIXTURE),
            );
        }

        let groups = scan_groups(dir.path());
        assert_eq!(groups.len(), 1, "{groups:?}");
        assert_eq!(groups[Path::new("Album")].len(), 8);
    }

    #[cfg(feature = "flac")]
    #[test]
    fn disc_directory_merges_with_tracks_in_parent() {
        let dir = tempfile::tempdir().unwrap();
        add_file(dir.path(), "Album/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/CD1/01.flac", Some(FLAC_FIXTURE));

        assert_eq!(
            scan_groups(dir.path()),
            group(&[("Album", &["Album/01.flac", "Album/CD1/01.flac"])])
        );
    }

    #[cfg(feature = "flac")]
    #[test]
    fn non_disc_directory_names_are_not_merged() {
        let dir = tempfile::tempdir().unwrap();
        // The pattern is anchored, requires digits and titles need a separator.
        add_file(dir.path(), "Album/CD/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/Disc/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/CD1 Bonus/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/My CD1/01.flac", Some(FLAC_FIXTURE));

        assert_eq!(
            scan_groups(dir.path()),
            group(&[
                ("Album/CD", &["Album/CD/01.flac"]),
                ("Album/CD1 Bonus", &["Album/CD1 Bonus/01.flac"]),
                ("Album/Disc", &["Album/Disc/01.flac"]),
                ("Album/My CD1", &["Album/My CD1/01.flac"]),
            ])
        );
    }

    #[cfg(feature = "flac")]
    #[test]
    fn disc_directory_in_scan_root_is_not_grouped_above_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let disc = dir.path().join("CD1");
        add_file(&disc, "01.flac", Some(FLAC_FIXTURE));

        let groups: Vec<_> = find_track_paths(disc.clone()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, disc);
        assert_eq!(groups[0].1.len(), 1);
    }

    #[cfg(feature = "flac")]
    #[test]
    fn disc_directories_of_different_albums_stay_separate() {
        let dir = tempfile::tempdir().unwrap();
        add_file(dir.path(), "A/CD1/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "B/CD1/01.flac", Some(FLAC_FIXTURE));

        assert_eq!(
            scan_groups(dir.path()),
            group(&[("A", &["A/CD1/01.flac"]), ("B", &["B/CD1/01.flac"])])
        );
    }

    #[cfg(feature = "flac")]
    #[test]
    fn disc_directories_and_siblings_form_one_group_regardless_of_walk_order() {
        let dir = tempfile::tempdir().unwrap();
        // Subdirectories sorting before and after the disc directories, and an intermediate
        // directory without tracks.
        add_file(dir.path(), "Album/CD1/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/CD2/01.flac", Some(FLAC_FIXTURE));
        add_file(dir.path(), "Album/Extras/01.flac", Some(FLAC_FIXTURE));
        add_file(
            dir.path(),
            "Album/Artwork/Scans/01.flac",
            Some(FLAC_FIXTURE),
        );
        add_file(dir.path(), "Album/Artwork/cover.jpg", None);
        add_file(dir.path(), "Other/01.flac", Some(FLAC_FIXTURE));

        assert_eq!(
            scan_groups(dir.path()),
            group(&[
                ("Album/Artwork/Scans", &["Album/Artwork/Scans/01.flac"]),
                ("Album", &["Album/CD1/01.flac", "Album/CD2/01.flac"]),
                ("Album/Extras", &["Album/Extras/01.flac"]),
                ("Other", &["Other/01.flac"]),
            ])
        );
    }

    #[cfg(feature = "flac")]
    #[test]
    fn discovery_is_lazy() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["A", "B", "C"] {
            add_file(dir.path(), &format!("{name}/01.flac"), Some(FLAC_FIXTURE));
        }

        let mut groups = find_track_paths(dir.path().to_path_buf());
        drop(groups.next().expect("first group"));

        // A group is yielded once the walk has moved on to the next directory, so at most one
        // directory is read ahead. Directories walked last must not have been read yet; files
        // added there now have to show up.
        let last_visited = ["A", "B", "C"]
            .into_iter()
            .map(|name| dir.path().join(name))
            .min()
            .unwrap();
        add_file(
            dir.path(),
            &format!(
                "{}/02.flac",
                last_visited.file_name().unwrap().to_str().unwrap()
            ),
            Some(FLAC_FIXTURE),
        );
        let remaining: Vec<_> = groups.collect();
        let last = remaining
            .iter()
            .find(|(path, _)| *path == last_visited)
            .unwrap();
        assert_eq!(last.1.len(), 2);
    }

    /// Run `dispatch_groups` on a separate thread. Every dispatched item and its permit is sent
    /// to the returned receiver. `pulled` counts items taken from the iterator.
    fn spawn_dispatcher(
        runtime: &Runtime,
        limiter: &Arc<Semaphore>,
        items: usize,
        pulled: &Arc<std::sync::atomic::AtomicUsize>,
    ) -> (
        std::thread::JoinHandle<()>,
        std::sync::mpsc::Receiver<(usize, OwnedSemaphorePermit)>,
    ) {
        use std::sync::atomic::Ordering;

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = runtime.handle().clone();
        let limiter = Arc::clone(limiter);
        let pulled = Arc::clone(pulled);
        let thread = std::thread::spawn(move || {
            let groups = (0..items).inspect(|_| {
                let _ = pulled.fetch_add(1, Ordering::SeqCst);
            });
            dispatch_groups(groups, &limiter, &handle, |item, permit| {
                tx.send((item, permit)).unwrap();
            });
        });
        (thread, rx)
    }

    const WAIT: std::time::Duration = std::time::Duration::from_secs(5);
    const SHORT_WAIT: std::time::Duration = std::time::Duration::from_millis(200);

    #[test]
    fn dispatch_pauses_discovery_until_permits_are_released() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .build()
            .unwrap();
        let limiter = Arc::new(Semaphore::new(2));
        let pulled = Arc::new(AtomicUsize::new(0));
        let (thread, rx) = spawn_dispatcher(&runtime, &limiter, 4, &pulled);

        let (first, first_permit) = rx.recv_timeout(WAIT).unwrap();
        let (second, _second_permit) = rx.recv_timeout(WAIT).unwrap();
        assert_eq!((first, second), (0, 1));

        // Limit reached: nothing is dispatched and nothing is pulled ahead of time.
        assert!(rx.recv_timeout(SHORT_WAIT).is_err());
        assert_eq!(pulled.load(Ordering::SeqCst), 2);

        // Releasing a permit lets exactly one more item through.
        drop(first_permit);
        let (third, _third_permit) = rx.recv_timeout(WAIT).unwrap();
        assert_eq!(third, 2);
        assert!(rx.recv_timeout(SHORT_WAIT).is_err());
        assert_eq!(pulled.load(Ordering::SeqCst), 3);

        // Closing the limiter stops discovery even though items are left.
        limiter.close();
        thread.join().unwrap();
        assert!(rx.try_recv().is_err());
        assert_eq!(pulled.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn dispatch_finishes_when_groups_are_exhausted() {
        use std::sync::atomic::AtomicUsize;

        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .build()
            .unwrap();
        let limiter = Arc::new(Semaphore::new(1));
        let pulled = Arc::new(AtomicUsize::new(0));
        let (thread, rx) = spawn_dispatcher(&runtime, &limiter, 3, &pulled);

        for expected in 0..3 {
            let (item, permit) = rx.recv_timeout(WAIT).unwrap();
            assert_eq!(item, expected);
            drop(permit);
        }
        thread.join().unwrap();
    }

    #[test]
    fn missing_scan_root_yields_no_groups() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_groups(&dir.path().join("does-not-exist")).is_empty());
    }
}
