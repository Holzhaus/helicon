// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Caching for MusicBrainz API queries.

use musicbrainz_rs_nova::entity::{
    release::Release as MusicBrainzRelease, search::SearchResult as MusicBrainzSearchResult,
};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use thiserror::Error;
use xdg::BaseDirectories;

/// Type alias for convenience.
type MusicBrainzReleaseSearchResult = MusicBrainzSearchResult<MusicBrainzRelease>;

/// Cache for MusicBrainz queries (to not use their API too much unnecessarily).
pub trait Cache {
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

/// Create the cache path for a MusicBrainz release with the given ID.
fn musicbrainz_release_path(mb_id: &str) -> PathBuf {
    Path::new("musicbrainz/release").join(format!("{mb_id}.json"))
}

/// Create the cache path for a MusicBrainz release with the given ID.
fn musicbrainz_search_query_path(query: &str, limit: u8, offset: u16) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    hasher.update([b'|', limit, b'|']);
    hasher.update(offset.to_be_bytes());
    let hash = hasher.finalize();
    Path::new("musicbrainz/release-search").join(format!("{hash:064x}.json"))
}

/// Convenience function to get a JSON-deserializable item with the given path from the cache.
fn get_from_cache<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T, CacheError> {
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