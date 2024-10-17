// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Path formatting and templating.
#![allow(dead_code)]

use crate::media::MediaLike;
use crate::release::ReleaseLike;
use crate::track::TrackLike;
use crate::Config;
use handlebars::{Handlebars, RenderError, TemplateError};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Characters that are forbidden in paths on Microsoft Windows (in addition to control characters).
#[cfg(target_os = "windows")]
const ILLEGAL_PATH_CHARS: &str = r#"\/:*?"<>|"#;

/// Characters that are forbidden in paths on Unices (in addition to control characters).
#[cfg(not(target_os = "windows"))]
const ILLEGAL_PATH_CHARS: &str = "/";

/// Strips control characters and escapes forbidden characters.
fn escape_path_chars(data: &str) -> String {
    data.chars()
        .filter(|c| !c.is_control())
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .map(|c| {
            if ILLEGAL_PATH_CHARS.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect::<String>()
}

/// Formatter for paths.
pub struct PathFormatter<'a>(Handlebars<'a>);

impl PathFormatter<'_> {
    /// Create a new path formatter.
    pub fn new(config: &Config) -> Result<Self, TemplateError> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(escape_path_chars);
        handlebars.register_template_string("album", &config.paths.album_format)?;
        handlebars.register_template_string("compilation", &config.paths.compilation_format)?;
        Ok(Self(handlebars))
    }

    /// Format a path with the given values.
    pub fn format(&self, values: &PathFormatterValues<'_>) -> Result<String, RenderError> {
        self.0.render("album", values)
    }
}

/// Possible values that can be used in a path formatter template.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathFormatterValues<'a> {
    /// The track's title.
    pub track_title: Option<Cow<'a, str>>,
    /// The track's artist (as credited for this track).
    pub track_artist: Option<Cow<'a, str>>,
    /// The track number (relative to the disc).
    pub track_number: Option<Cow<'a, str>>,
    /// The number of tracks on the disc).
    pub track_count: Option<usize>,
    /// The album's title.
    pub album_title: Option<Cow<'a, str>>,
    /// The album's artist (as credited for this release).
    pub album_artist: Option<Cow<'a, str>>,
    /// The disc number.
    pub disc_number: Option<u32>,
    /// The total number of discs that are part of this release.
    pub disc_count: Option<usize>,
}

impl<'a> PathFormatterValues<'a> {
    /// Assign fields from a [`ReleaseLike`] object.
    pub fn with_release(mut self, release: &'a impl ReleaseLike) -> Self {
        self.album_title = release.release_title();
        self.album_artist = release.release_artist();
        self.disc_count = Some(release.media().count());
        self
    }

    /// Assign fields from a [`MediaLike`] object.
    pub fn with_media(mut self, media: &'a impl MediaLike) -> Self {
        self.disc_number = media.disc_number();
        self.track_count = Some(media.media_tracks().count());
        self
    }

    /// Assign fields from a [`TrackLike`] object.
    pub fn with_track(mut self, track: &'a impl TrackLike) -> Self {
        self.track_title = track.track_title();
        self.track_artist = track.track_artist();
        self.track_number = track.track_number();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use musicbrainz_rs_nova::entity::release::Release as MusicBrainzRelease;

    const MUSICBRAINZ_RELEASE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/musicbrainz/release.json"
    ));

    #[test]
    fn test_album_path() {
        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let media = release.media().next().unwrap();
        let track = media.media_tracks().next().unwrap();

        let config = Config::default();
        let formatter = PathFormatter::new(&config).unwrap();
        let values = PathFormatterValues::default()
            .with_release(&release)
            .with_media(media)
            .with_track(track);

        let output = formatter.format(&values).unwrap();
        assert_eq!(
            output,
            "The Ahmad Jamal Trio/Ahmad Jamal at the Pershing: But Not for Me/1-1 - But Not for Me"
        );
    }
}
