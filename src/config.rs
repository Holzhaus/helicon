// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Configuration utils.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Encountered when the configuration cannot be loaded.
#[derive(Error, Debug)]
#[error("Configuration Error: {0}")]
pub struct ConfigError(#[from] toml::de::Error);

/// Default configuration TOML string.
const DEFAULT_CONFIG: &str = include_str!("default_config.toml");

/// Weight for a distance calculation.
pub type DistanceWeight = f64;

/// Represents a piece of configuration that can be merged with another one.
trait MergeableConfig {
    /// Merge this configuration object with another one, taking values not set in this object from
    /// the other one (if present).
    fn merge(&self, other: &Self) -> Self;
}

/// Weights for track distance calculation.
#[expect(missing_copy_implementations)]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct TrackDistanceWeights {
    /// Track title weight.
    pub track_title: Option<DistanceWeight>,
    /// Track artist weight.
    pub track_artist: Option<DistanceWeight>,
    /// Track number weight.
    pub track_number: Option<DistanceWeight>,
    /// Track length weight.
    pub track_length: Option<DistanceWeight>,
    /// MusicBrainz Recording ID weight.
    pub musicbrainz_recording_id: Option<DistanceWeight>,
}

impl MergeableConfig for TrackDistanceWeights {
    fn merge(&self, other: &Self) -> Self {
        TrackDistanceWeights {
            track_title: self.track_title.or(other.track_title),
            track_artist: self.track_artist.or(other.track_artist),
            track_number: self.track_number.or(other.track_number),
            track_length: self.track_length.or(other.track_length),
            musicbrainz_recording_id: self
                .musicbrainz_recording_id
                .or(other.musicbrainz_recording_id),
        }
    }
}

/// Weights  for release distance calculation.
#[expect(missing_copy_implementations)]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ReleaseDistanceWeights {
    /// Release title weight.
    pub release_title: Option<DistanceWeight>,
    /// Release artist weight.
    pub release_artist: Option<DistanceWeight>,
    /// MusicBrainz Release ID weight.
    pub musicbrainz_release_id: Option<DistanceWeight>,
    /// Media Format weight.
    pub media_format: Option<DistanceWeight>,
    /// Record label weight.
    pub record_label: Option<DistanceWeight>,
    /// Catalog number weight.
    pub catalog_number: Option<DistanceWeight>,
    /// Barcode weight.
    pub barcode: Option<DistanceWeight>,
    /// Overall track assignment weight.
    pub track_assignment: Option<DistanceWeight>,
}

impl MergeableConfig for ReleaseDistanceWeights {
    fn merge(&self, other: &Self) -> Self {
        ReleaseDistanceWeights {
            release_title: self.release_title.or(other.release_title),
            release_artist: self.release_artist.or(other.release_artist),
            musicbrainz_release_id: self.musicbrainz_release_id.or(other.musicbrainz_release_id),
            media_format: self.media_format.or(other.media_format),
            record_label: self.record_label.or(other.record_label),
            catalog_number: self.catalog_number.or(other.catalog_number),
            barcode: self.barcode.or(other.barcode),
            track_assignment: self.track_assignment.or(other.track_assignment),
        }
    }
}

/// Weight configuration.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct DistanceWeights {
    /// Weights for track distance calculation.
    pub track: TrackDistanceWeights,
    /// Weights for release distance calculation.
    pub release: ReleaseDistanceWeights,
}

impl MergeableConfig for DistanceWeights {
    fn merge(&self, other: &Self) -> Self {
        DistanceWeights {
            track: self.track.merge(&other.track),
            release: self.release.merge(&other.release),
        }
    }
}

/// Configuration for MusicBrainz lookups.
#[expect(missing_copy_implementations)]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct LookupConfig {
    /// Number of concurrent connections to use.
    pub connection_limit: Option<usize>,
    /// Do not fetch more than this number of candidate releases from MusicBrainz.
    ///
    /// Use `0` to disable this limit.
    pub release_candidate_limit: Option<u8>,
}

impl MergeableConfig for LookupConfig {
    fn merge(&self, other: &Self) -> Self {
        LookupConfig {
            connection_limit: self.connection_limit.or(other.connection_limit),
            release_candidate_limit: self
                .release_candidate_limit
                .or(other.release_candidate_limit)
                .filter(|&x| x != 0),
        }
    }
}

/// The main configuration struct.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Configuration for track/release lookup.
    pub lookup: LookupConfig,
    /// Weight configuration.
    pub weights: DistanceWeights,
}

impl Default for Config {
    fn default() -> Self {
        Self::load_default().expect("Failed to load default config")
    }
}

impl MergeableConfig for Config {
    /// Merge this configuration object with another one, taking values not set in this object from
    /// the other one (if present).
    fn merge(&self, other: &Self) -> Self {
        Config {
            lookup: self.lookup.merge(&other.lookup),
            weights: self.weights.merge(&other.weights),
        }
    }
}

impl Config {
    /// Load the configuration from a string slice.
    fn load_from_str(text: &str) -> Result<Self, ConfigError> {
        let config = toml::from_str(text)?;
        Ok(config)
    }

    /// Load the default configuration.
    fn load_default() -> Result<Self, ConfigError> {
        Self::load_from_str(DEFAULT_CONFIG)
    }

    /// Load the configuration from a file located at the given path.
    ///
    /// # Errors
    ///
    /// This method can fail if the file cannot be accessed or if it contains malformed
    /// configuration markup.
    pub fn load_from_path<T: AsRef<Path>>(path: T) -> crate::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let config = Self::load_from_str(&text)?;
        Ok(config)
    }

    /// Merge this configuration struct with the default values.
    #[must_use]
    pub fn with_defaults(&self) -> Self {
        let default = Self::default();
        self.merge(&default)
    }
}
