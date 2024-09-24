// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Caching for MusicBrainz API queries.

use musicbrainz_rs_nova::entity::{
    release::Release as MusicBrainzRelease, release_group::ReleaseGroup as MusicBrainzReleaseGroup,
    search::SearchResult as MusicBrainzSearchResult,
};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use xdg::BaseDirectories;

/// Type alias for convenience.
type MusicBrainzReleaseSearchResult = MusicBrainzSearchResult<MusicBrainzRelease>;

/// Cache for MusicBrainz queries (to not use their API too much unnecessarily).
pub trait Cache {
    /// Get a list of all release search result cache files.
    fn cached_release_search_results(&self) -> Vec<PathBuf>;

    /// Get a release from the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if a cache miss occurred or the cache file could not be read or the
    /// deserialization failed.
    fn get_release_search_result(
        &self,
        query: &str,
        limit: u8,
        offset: u16,
    ) -> Result<MusicBrainzReleaseSearchResult, CacheError>;

    /// Insert a release into the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file could not be written or the serialization failed.
    fn insert_release_search_result(
        &self,
        query: &str,
        limit: u8,
        offset: u16,
        result: &MusicBrainzReleaseSearchResult,
    ) -> Result<(), CacheError>;

    /// Get a list of all release cache files.
    fn cached_releases(&self) -> Vec<PathBuf>;

    /// Get a release from the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if a cache miss occurred or the cache file could not be read or the
    /// deserialization failed.
    fn get_release(&self, mb_id: &str) -> Result<MusicBrainzRelease, CacheError>;

    /// Insert a release into the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file could not be written or the serialization failed.
    fn insert_release(&self, mb_id: &str, release: &MusicBrainzRelease) -> Result<(), CacheError>;

    /// Get a list of all release group cache files.
    fn cached_release_groups(&self) -> Vec<PathBuf>;

    /// Get a release group from the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if a cache miss occurred or the cache file could not be read or the
    /// deserialization failed.
    fn get_release_group(&self, mb_id: &str) -> Result<MusicBrainzReleaseGroup, CacheError>;

    /// Insert a release group into the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file could not be written or the serialization failed.
    fn insert_release_group(
        &self,
        mb_id: &str,
        release: &MusicBrainzReleaseGroup,
    ) -> Result<(), CacheError>;
}

/// Cache Error.
#[derive(Error, Debug)]
pub enum CacheError {
    /// Item was not found in cache.
    #[error("Cache Miss")]
    CacheMiss,
    /// I/O Error.
    #[error("Input/Output error ({:?})", .0)]
    Io(#[from] io::Error),
    /// JSON (De-)Serialization Error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Path under which the cached releases are stored.
const MUSICBRAINZ_RELEASE_PATH_PREFIX: &str = "musicbrainz/release";

/// Path under which the cached release groups are stored.
const MUSICBRAINZ_RELEASE_GROUP_PATH_PREFIX: &str = "musicbrainz/release-group";

/// Path under which the cached release search results are stored.
const MUSICBRAINZ_RELEASE_SEARCH_RESULTS_PATH_PREFIX: &str = "musicbrainz/release-search";

/// Maximum age of a a cache entry after which it expires.
const MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 7);

/// Create the cache path for a MusicBrainz release with the given ID.
fn musicbrainz_release_path(mb_id: &str) -> PathBuf {
    Path::new(MUSICBRAINZ_RELEASE_PATH_PREFIX).join(format!("{mb_id}.json"))
}

/// Create the cache path for a MusicBrainz release with the given ID.
fn musicbrainz_release_group_path(mb_id: &str) -> PathBuf {
    Path::new(MUSICBRAINZ_RELEASE_GROUP_PATH_PREFIX).join(format!("{mb_id}.json"))
}

/// Create the cache path for a MusicBrainz release with the given ID.
fn musicbrainz_search_query_path(query: &str, limit: u8, offset: u16) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    hasher.update([b'|', limit, b'|']);
    hasher.update(offset.to_be_bytes());
    let hash = hasher.finalize();
    Path::new(MUSICBRAINZ_RELEASE_SEARCH_RESULTS_PATH_PREFIX).join(format!("{hash:064x}.json"))
}

/// Convenience function to get a JSON-deserializable item with the given path from the cache.
fn get_from_cache<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T, CacheError> {
    let cache_age = path
        .as_ref()
        .metadata()?
        .modified()
        .ok()
        .and_then(|time| time.elapsed().ok())
        .unwrap_or(Duration::MAX);
    // TODO: Make this configurable.
    if cache_age > MAX_AGE {
        std::fs::remove_file(path)?;
        return Err(CacheError::CacheMiss);
    }

    let f = File::open(path)?;
    let reader = BufReader::new(f);
    Ok(serde_json::from_reader(reader)?)
}

/// Convenience function to insert a JSON-deserializable item with the given path into cache.
fn insert_into_cache<T: Serialize, P: AsRef<Path>>(path: P, item: &T) -> Result<(), CacheError> {
    let f = File::create(path)?;
    let writer = BufWriter::new(f);
    Ok(serde_json::to_writer(writer, item)?)
}

impl Cache for BaseDirectories {
    fn cached_releases(&self) -> Vec<PathBuf> {
        self.list_cache_files(MUSICBRAINZ_RELEASE_PATH_PREFIX)
    }

    fn get_release(&self, mb_id: &str) -> Result<MusicBrainzRelease, CacheError> {
        let path = self
            .find_cache_file(musicbrainz_release_path(mb_id))
            .ok_or(CacheError::CacheMiss)?;
        get_from_cache(path)
    }

    fn insert_release(&self, mb_id: &str, release: &MusicBrainzRelease) -> Result<(), CacheError> {
        let path = self.place_cache_file(musicbrainz_release_path(mb_id))?;
        insert_into_cache(path, release)
    }

    fn cached_release_groups(&self) -> Vec<PathBuf> {
        self.list_cache_files(MUSICBRAINZ_RELEASE_GROUP_PATH_PREFIX)
    }

    fn get_release_group(&self, mb_id: &str) -> Result<MusicBrainzReleaseGroup, CacheError> {
        let path = self
            .find_cache_file(musicbrainz_release_group_path(mb_id))
            .ok_or(CacheError::CacheMiss)?;
        get_from_cache(path)
    }

    fn insert_release_group(
        &self,
        mb_id: &str,
        release: &MusicBrainzReleaseGroup,
    ) -> Result<(), CacheError> {
        let path = self.place_cache_file(musicbrainz_release_group_path(mb_id))?;
        insert_into_cache(path, release)
    }

    fn cached_release_search_results(&self) -> Vec<PathBuf> {
        self.list_cache_files(MUSICBRAINZ_RELEASE_SEARCH_RESULTS_PATH_PREFIX)
    }

    fn get_release_search_result(
        &self,
        query: &str,
        limit: u8,
        offset: u16,
    ) -> Result<MusicBrainzReleaseSearchResult, CacheError> {
        let path = self
            .find_cache_file(musicbrainz_search_query_path(query, limit, offset))
            .ok_or(CacheError::CacheMiss)?;
        get_from_cache(path)
    }

    fn insert_release_search_result(
        &self,
        query: &str,
        limit: u8,
        offset: u16,
        result: &MusicBrainzReleaseSearchResult,
    ) -> Result<(), CacheError> {
        let path = self.place_cache_file(musicbrainz_search_query_path(query, limit, offset))?;
        // FIXME: This doesn't work due to https://github.com/RustyNova016/musicbrainz_rs_nova/issues/33
        insert_into_cache(path, result)
    }
}
