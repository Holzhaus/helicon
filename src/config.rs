// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Configuration utils.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Encountered when the configuration cannot be loaded.
#[derive(Error, Debug)]
#[error("Configuration Error: {0}")]
pub enum ConfigError {
    /// The configuration failed to load.
    LoadingFailed(#[from] config::ConfigError),
    /// The configuration is invalid (e.g., due to missing values).
    Invalid(#[from] toml::de::Error),
}

/// Default configuration TOML string.
const DEFAULT_CONFIG: &str = include_str!("default_config.toml");

/// Weight for a distance calculation.
pub type DistanceWeight = f64;

/// Weights for track distance calculation.
#[expect(missing_copy_implementations)]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct TrackDistanceWeights {
    /// Track title weight.
    pub track_title: DistanceWeight,
    /// Track artist weight.
    pub track_artist: DistanceWeight,
    /// Track number weight.
    pub track_number: DistanceWeight,
    /// Track length weight.
    pub track_length: DistanceWeight,
    /// MusicBrainz Recording ID weight.
    pub musicbrainz_recording_id: DistanceWeight,
}

/// Weights  for release distance calculation.
#[expect(missing_copy_implementations)]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ReleaseDistanceWeights {
    /// Release title weight.
    pub release_title: DistanceWeight,
    /// Release artist weight.
    pub release_artist: DistanceWeight,
    /// MusicBrainz Release ID weight.
    pub musicbrainz_release_id: DistanceWeight,
    /// Media Format weight.
    pub media_format: DistanceWeight,
    /// Record label weight.
    pub record_label: DistanceWeight,
    /// Catalog number weight.
    pub catalog_number: DistanceWeight,
    /// Barcode weight.
    pub barcode: DistanceWeight,
    /// Overall track assignment weight.
    pub track_assignment: DistanceWeight,
}

/// Weight configuration.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct DistanceWeights {
    /// Weights for track distance calculation.
    pub track: TrackDistanceWeights,
    /// Weights for release distance calculation.
    pub release: ReleaseDistanceWeights,
}

/// Configuration for MusicBrainz lookups.
#[expect(missing_copy_implementations)]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct LookupConfig {
    /// Number of concurrent connections to use.
    pub connection_limit: usize,
    /// Do not fetch more than this number of candidate releases from MusicBrainz.
    ///
    /// Must be a number between 1 and 100.
    pub release_candidate_limit: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crossterm::style::Color")]
#[serde(rename_all = "snake_case")]
#[allow(clippy::missing_docs_in_private_items)]
enum ColorDef {
    Reset,
    Black,
    DarkGrey,
    Red,
    DarkRed,
    Green,
    DarkGreen,
    Yellow,
    DarkYellow,
    Blue,
    DarkBlue,
    Magenta,
    DarkMagenta,
    Cyan,
    DarkCyan,
    White,
    Grey,
    Rgb { r: u8, g: u8, b: u8 },
    AnsiValue(u8),
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "crossterm::style::Attribute")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
#[allow(clippy::missing_docs_in_private_items)]
enum AttributeDef {
    Reset,
    Bold,
    Dim,
    Italic,
    Underlined,
    DoubleUnderlined,
    Undercurled,
    Underdotted,
    Underdashed,
    SlowBlink,
    RapidBlink,
    Reverse,
    Hidden,
    CrossedOut,
    Fraktur,
    NoBold,
    NormalIntensity,
    NoItalic,
    NoUnderline,
    NoBlink,
    NoReverse,
    NoHidden,
    NotCrossedOut,
    Framed,
    Encircled,
    OverLined,
    NotFramedOrEncircled,
    NotOverLined,
}

/// Wrapper for crossterm's `Color` type that supports Serde.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct TextColor(#[serde(with = "ColorDef")] crossterm::style::Color);
impl From<TextColor> for crossterm::style::Color {
    fn from(value: TextColor) -> Self {
        value.0
    }
}

/// Wrapper for crossterm's `Attribute` type that supports Serde.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct TextAttribute(#[serde(with = "AttributeDef")] crossterm::style::Attribute);
impl From<&TextAttribute> for crossterm::style::Attribute {
    fn from(value: &TextAttribute) -> Self {
        value.0
    }
}

/// Style definition that supports Serde.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyleConfig {
    /// Text foreground color.
    foreground_color: Option<TextColor>,
    /// Text background color.
    background_color: Option<TextColor>,
    /// Text underline color.
    underline_color: Option<TextColor>,
    /// Text attributes.
    attributes: Option<Vec<TextAttribute>>,
}

impl From<&TextStyleConfig> for crossterm::style::ContentStyle {
    fn from(value: &TextStyleConfig) -> crossterm::style::ContentStyle {
        let mut content_style = crossterm::style::ContentStyle::new();
        content_style.foreground_color = value.foreground_color.map(TextColor::into);
        content_style.background_color = value.background_color.map(TextColor::into);
        content_style.underline_color = value.underline_color.map(TextColor::into);
        content_style.attributes = value.attributes.iter().flat_map(|v| v.iter()).fold(
            crossterm::style::Attributes::none(),
            |attributes, attribute| attributes.with(attribute.into()),
        );
        content_style
    }
}

impl TextStyleConfig {
    /// Apply the style to text.
    pub fn apply<D: std::fmt::Display>(&self, val: D) -> crossterm::style::StyledContent<D> {
        crossterm::style::ContentStyle::from(self).apply(val)
    }
}

/// Configuration for the user interface.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StringDiffStyleConfig {
    /// Style for values that are present when the other one is missing.
    pub present: TextStyleConfig,
    /// Style for values that are missing when the other one is present.
    pub missing: TextStyleConfig,
    /// Style for text that is equal in both values.
    pub equal: TextStyleConfig,
    /// Style for text that is deleted from the old value.
    pub delete: TextStyleConfig,
    /// Style for text that is inserted into the new value.
    pub insert: TextStyleConfig,
    /// Style for text that is replaced in the old value.
    pub replace_old: TextStyleConfig,
    /// Style for text that is replaced in the new value.
    pub replace_new: TextStyleConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnmatchedTrackStyleConfig {
    /// Prefix that is displayed at the start of the line.
    pub prefix: String,
    /// Style of the prefix that is displayed at the start of the line.
    pub prefix_style: TextStyleConfig,
    /// Headline style.
    pub headline_style: TextStyleConfig,
    /// Track number style.
    pub track_number_style: TextStyleConfig,
    /// Track title style.
    pub track_title_style: TextStyleConfig,
}

/// Configuration for the user interface.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandidateDetails {
    /// Indent string for the tracklist.
    pub tracklist_indent: String,
    /// Separator between the left and the right side of the tracklist.
    pub tracklist_separator: String,
    /// Indent string for additional tags.
    pub tracklist_extra_indent: String,
    /// Separator between the left and the right side for additional tags.
    pub tracklist_extra_separator: String,
    /// Release artist and title style.
    pub release_artist_and_title_style: TextStyleConfig,
    /// Release metadata style.
    pub release_meta_style: TextStyleConfig,
    /// Disc title style.
    pub disc_title_style: TextStyleConfig,
    /// Track number style.
    pub track_number_style: TextStyleConfig,
    /// Track number style for defaulted (missing) track numbers.
    pub track_number_style_default: TextStyleConfig,
    /// Changed value indicator style.
    pub changed_value_style: TextStyleConfig,
    /// Styles for residual tracks.
    pub unmatched_tracks_residual: UnmatchedTrackStyleConfig,
    /// Styles for missing tracks.
    pub unmatched_tracks_missing: UnmatchedTrackStyleConfig,
    /// Style for the selection
    pub action_style: TextStyleConfig,
    /// Additional attributes for the candidate similarity in the selection list.
    pub candidate_similarity_style: TextStyleConfig,
    /// Prefix for the candidate similarity in the selection list.
    pub candidate_similarity_prefix: String,
    /// Style of the prefix for the candidate similarity in the selection list.
    pub candidate_similarity_prefix_style: TextStyleConfig,
    /// Suffix for the candidate similarity in the selection list.
    pub candidate_similarity_suffix: String,
    /// Style of the suffix for the candidate similarity in the selection list.
    pub candidate_similarity_suffix_style: TextStyleConfig,
    /// Styles for string diffs.
    pub string_diff_style: StringDiffStyleConfig,
}

/// Configuration for the user interface.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UiConfig {
    /// Default width of the terminal that is assumed if it cannot be detected.
    pub default_terminal_width: usize,
    /// Maximum terminal width to use. If the terminal is wider, this configured width will be
    /// used.
    pub max_terminal_width: Option<usize>,
    /// Styles for the candidate details view.
    pub candidate_details: CandidateDetails,
}

/// The main configuration struct.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Configuration for track/release lookup.
    pub lookup: LookupConfig,
    /// Weight configuration.
    pub weights: DistanceWeights,
    /// UI configuration.
    pub user_interface: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self::load_default().expect("Failed to parse default configuration")
    }
}

/// Builder pattern for the configuration.
mod builder {
    use super::{Config, ConfigError, DEFAULT_CONFIG};
    use config::{
        builder::DefaultState, Config as BaseConfig, ConfigBuilder as BaseConfigBuilder, File,
        FileFormat,
    };
    use std::path::Path;

    /// Builder for the configuration object.
    #[derive(Debug)]
    pub struct ConfigBuilder(BaseConfigBuilder<DefaultState>);

    impl Default for ConfigBuilder {
        fn default() -> Self {
            Self(BaseConfig::builder())
        }
    }

    impl ConfigBuilder {
        /// Add a file to be loaded to the configuration builder. Files added later will override
        /// values from previous files.
        pub fn with_file<P: AsRef<Path>>(mut self, path: P) -> Self {
            self.0 = self
                .0
                .add_source(File::from(path.as_ref()).format(FileFormat::Toml));
            self
        }

        /// Add a file to be loaded to the configuration builder. Files added later will override
        /// values from previous files.
        pub fn with_str<S: AsRef<str>>(mut self, value: S) -> Self {
            self.0 = self
                .0
                .add_source(File::from_str(value.as_ref(), FileFormat::Toml));
            self
        }

        /// Add the default configuration to the configuration builder.
        pub fn with_defaults(self) -> Self {
            self.with_str(DEFAULT_CONFIG)
        }

        /// Actually load the configuration from the builder.
        pub fn build(self) -> Result<Config, ConfigError> {
            Ok(self.0.build().unwrap().try_deserialize::<Config>()?)
        }
    }
}

impl Config {
    /// Load the configuration from a file located at the given path.
    ///
    /// # Errors
    ///
    /// This method can fail if the file cannot be accessed or if it contains malformed
    /// configuration markup.
    #[must_use]
    pub fn builder() -> builder::ConfigBuilder {
        builder::ConfigBuilder::default()
    }

    /// Load the configuration from a string slice.
    ///
    /// # Errors
    ///
    /// This method can fail if the configuration is missing values or malformed.
    fn load_from_str(text: &str) -> Result<Self, ConfigError> {
        let config = toml::from_str(text)?;
        Ok(config)
    }

    /// Load the default configugration.
    ///
    /// # Errors
    ///
    /// This method can fail if the configuration is missing values or malformed.
    fn load_default() -> Result<Self, ConfigError> {
        Self::load_from_str(DEFAULT_CONFIG)
    }
}
