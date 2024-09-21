// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Error and result types.

use std::io;
use thiserror::Error;

/// Main error type.
#[derive(Error, Debug)]
pub enum ErrorType {
    /// Configuration error.
    #[error("Configuration Error ({0})")]
    Config(#[from] crate::config::ConfigError),
    /// I/O Error.
    #[error("Input/Output error ({:?})", .0)]
    Io(#[from] io::Error),
    /// File has an unknown file extension.
    #[error("File has unknown file type")]
    UnknownFileType,
    /// A MusicBrainz API request failed.
    #[error("API request failed")]
    Request(#[from] musicbrainz_rs_nova::Error),
    /// A MusicBrainz API request failed.
    #[error("MusicBrainz lookup failed")]
    MusicBrainzLookupFailed(&'static str),
    /// Errors raised by the [`id3`] crate.
    #[cfg(feature = "id3")]
    #[error("Failed to read ID3 tag")]
    Id3(#[from] id3::Error),
    /// Errors raised by the [`metaflac`] crate.
    #[cfg(feature = "flac")]
    #[error("Failed to read FLAC tag")]
    Flac(#[from] metaflac::Error),
}

/// Convenience type.
pub type Result<T> = std::result::Result<T, ErrorType>;
