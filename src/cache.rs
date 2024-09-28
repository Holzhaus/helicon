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

/// Maximum age of a a cache entry after which it expires.
const MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 7);

/// Cache for MusicBrainz queries (to not use their API too much unnecessarily).
#[derive(Debug, Clone)]
pub struct Cache(BaseDirectories);

impl Cache {
    /// Create a new cache struct.
    #[must_use]
    pub fn new(base_dirs: BaseDirectories) -> Self {
        Self(base_dirs)
    }

    /// Get a JSON-deserializable item with the given path from the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if a cache miss occurred or the cache file could not be read or the
    /// deserialization failed.
    pub fn get_item<'a, T: Cacheable<'a> + DeserializeOwned>(
        &self,
        key: T::Key,
    ) -> Result<T, CacheError> {
        let item_path = T::cache_path(key);
        let path = self
            .0
            .find_cache_file(item_path)
            .ok_or(CacheError::CacheMiss)?;
        let cache_age = path
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

    /// Insert a JSON-deserializable item with the given path into cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file could not be written or the serialization failed.
    pub fn insert_item<'a, T: Cacheable<'a> + Serialize>(
        &self,
        key: T::Key,
        item: &T,
    ) -> Result<(), CacheError> {
        let item_path = T::cache_path(key);
        let path = self.0.place_cache_file(item_path)?;
        let f = File::create(path)?;
        let writer = BufWriter::new(f);
        Ok(serde_json::to_writer(writer, item)?)
    }

    /// Get a tuple `(item_count, total_size_in_bytes)` for items at given cache path.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file metadata could not be read.
    pub fn get_stats<'a, T: Cacheable<'a>>(&self) -> Result<(usize, u64), CacheError> {
        let items = self.0.list_cache_files(T::CACHE_DIRECTORY);
        let item_count = items.len();
        let item_size = items
            .iter()
            .map(|file| file.metadata().map(|metadata| metadata.len()))
            .sum::<io::Result<u64>>()?;
        Ok((item_count, item_size))
    }
}

/// Marks an item as cacheable.
pub trait Cacheable<'a> {
    /// Type of the cache key.
    type Key;

    /// Directory inside the cache where items of this type are stored.
    const CACHE_DIRECTORY: &'static str;

    /// The cache path for the given key.
    fn cache_path(key: Self::Key) -> PathBuf;
}

impl<'a> Cacheable<'a> for MusicBrainzRelease {
    type Key = &'a str;

    const CACHE_DIRECTORY: &'static str = "musicbrainz/release";

    fn cache_path(mb_id: Self::Key) -> PathBuf {
        Path::new(Self::CACHE_DIRECTORY).join(format!("{mb_id}.json"))
    }
}

impl<'a> Cacheable<'a> for MusicBrainzReleaseGroup {
    type Key = &'a str;

    const CACHE_DIRECTORY: &'static str = "musicbrainz/release-group";

    fn cache_path(mb_id: Self::Key) -> PathBuf {
        Path::new(Self::CACHE_DIRECTORY).join(format!("{mb_id}.json"))
    }
}

// FIXME: This doesn't work due to <https://github.com/RustyNova016/musicbrainz_rs_nova/issues/33>.
impl<'a> Cacheable<'a> for MusicBrainzReleaseSearchResult {
    type Key = (&'a str, u8, u16);

    const CACHE_DIRECTORY: &'static str = "musicbrainz/release-search";

    fn cache_path((query, limit, offset): (&str, u8, u16)) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        hasher.update([b'|', limit, b'|']);
        hasher.update(offset.to_be_bytes());
        let hash = hasher.finalize();
        Path::new(Self::CACHE_DIRECTORY).join(format!("{hash:064x}.json"))
    }
}
