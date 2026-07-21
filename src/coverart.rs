// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Cover art fetching and assignment from MusicBrainz.

use std::borrow::Cow;

/// Represents cover art image data with metadata.
#[derive(Debug, Clone)]
pub struct CoverArt {
    /// The raw image data.
    pub data: Vec<u8>,
    /// MIME type of the image (e.g., "image/jpeg", "image/png").
    pub mime_type: String,
}

impl CoverArt {
    /// Create new cover art from raw image data and MIME type.
    pub fn new(data: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self {
            data,
            mime_type: mime_type.into(),
        }
    }

    /// Get the file extension based on the MIME type.
    pub fn extension(&self) -> &'static str {
        match self.mime_type.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "jpg", // Default to jpg
        }
    }

    /// Check if this is a valid cover art image type.
    pub fn is_valid(&self) -> bool {
        matches!(
            self.mime_type.as_str(),
            "image/jpeg" | "image/png" | "image/gif" | "image/webp"
        )
    }
}

/// Types of cover art that can be fetched from MusicBrainz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverArtType {
    /// Front cover art.
    Front,
    /// Back cover art.
    Back,
    /// Any available cover art type.
    Any,
}

impl CoverArtType {
    /// Get the string representation for MusicBrainz API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Front => "Front",
            Self::Back => "Back",
            Self::Any => "",
        }
    }
}

/// Configuration for cover art fetching.
#[derive(Debug, Clone)]
pub struct CoverArtConfig {
    /// The type of cover art to fetch.
    pub art_type: CoverArtType,
    /// Maximum size to fetch (in bytes). If None, no limit.
    pub max_size: Option<usize>,
}

impl Default for CoverArtConfig {
    fn default() -> Self {
        Self {
            art_type: CoverArtType::Front,
            max_size: Some(10 * 1024 * 1024), // 10 MB default limit
        }
    }
}

/// Result type for cover art operations.
pub type CoverArtResult<T> = Result<T, CoverArtError>;

/// Errors that can occur during cover art operations.
#[derive(Debug)]
pub enum CoverArtError {
    /// Cover art not found for this release.
    NotFound,
    /// Cover art is too large.
    TooLarge,
    /// Invalid MIME type.
    InvalidMimeType(String),
    /// Network error fetching cover art.
    FetchError(String),
    /// IO error writing cover art.
    IoError(std::io::Error),
}

impl std::fmt::Display for CoverArtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Cover art not found"),
            Self::TooLarge => write!(f, "Cover art is too large"),
            Self::InvalidMimeType(mime) => write!(f, "Invalid MIME type: {}", mime),
            Self::FetchError(msg) => write!(f, "Failed to fetch cover art: {}", msg),
            Self::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for CoverArtError {}

impl From<std::io::Error> for CoverArtError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverart_extension() {
        let jpeg = CoverArt::new(vec![0xFF, 0xD8], "image/jpeg");
        assert_eq!(jpeg.extension(), "jpg");

        let png = CoverArt::new(vec![0x89, 0x50], "image/png");
        assert_eq!(png.extension(), "png");

        let webp = CoverArt::new(vec![0x52, 0x49], "image/webp");
        assert_eq!(webp.extension(), "webp");
    }

    #[test]
    fn test_coverart_is_valid() {
        let valid_jpeg = CoverArt::new(vec![0xFF, 0xD8], "image/jpeg");
        assert!(valid_jpeg.is_valid());

        let invalid = CoverArt::new(vec![0x00], "image/invalid");
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_coverart_type_as_str() {
        assert_eq!(CoverArtType::Front.as_str(), "Front");
        assert_eq!(CoverArtType::Back.as_str(), "Back");
        assert_eq!(CoverArtType::Any.as_str(), "");
    }

    #[test]
    fn test_coverart_config_default() {
        let config = CoverArtConfig::default();
        assert_eq!(config.art_type, CoverArtType::Front);
        assert_eq!(config.max_size, Some(10 * 1024 * 1024));
    }
}
