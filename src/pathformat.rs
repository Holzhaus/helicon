// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Path formatting and templating.
#![allow(dead_code)]

use crate::config::PathTemplateConfig;
use crate::media::MediaLike;
use crate::release::ReleaseLike;
use crate::track::TrackLike;
use handlebars::{handlebars_helper, Handlebars, RenderError, Template, TemplateError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// Return the width of a base-10 `usize` integer as usize (if possible).
fn usize_width(integer: usize) -> Option<usize> {
    if integer == 0 {
        return Some(1);
    }

    usize::try_from(integer.ilog10() + 1).ok()
}

/// Configuration for the [`PathFormatter`] object.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(try_from = "PathTemplateConfig")]
#[serde(into = "PathTemplateConfig")]
pub struct PathTemplate {
    /// The original config (for serialization).
    config: PathTemplateConfig,
    /// Format for album file paths.
    pub album_format: Template,
    /// Format for compilation file paths.
    pub compilation_format: Template,
}

impl PathTemplate {
    /// Get the [`PathFormatter`] from this configuration.
    pub fn formatter(&self) -> PathFormatter {
        PathFormatter::from(self)
    }
}

impl TryFrom<PathTemplateConfig> for PathTemplate {
    type Error = TemplateError;

    fn try_from(config: PathTemplateConfig) -> Result<Self, Self::Error> {
        let album_format = Template::compile(&config.album_format)?;
        let compilation_format = Template::compile(&config.album_format)?;
        Ok(Self {
            config,
            album_format,
            compilation_format,
        })
    }
}

impl From<PathTemplate> for PathTemplateConfig {
    fn from(template: PathTemplate) -> Self {
        template.config
    }
}

handlebars_helper!(helper_zfill: |value: Value, { width: usize = 0 }| {
    match value {
        Value::Number(number) => {
            format!("{number:0>width$}", number = number.to_string())
        }
        Value::String(string) => {
            format!("{string:0>width$}")
        }
        _ => unreachable!()
    }
});

/// Formatter for paths.
#[derive(Debug, Clone)]
pub struct PathFormatter(Handlebars<'static>);

impl From<&PathTemplate> for PathFormatter {
    fn from(template: &PathTemplate) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(escape_path_chars);
        handlebars.register_helper("zfill", Box::new(helper_zfill));
        handlebars.register_template("album", template.album_format.clone());
        handlebars.register_template("compilation", template.compilation_format.clone());
        Self(handlebars)
    }
}

impl PathFormatter {
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
    /// The track index (relative to the disc).
    pub track_index: Option<usize>,
    /// The number of tracks on the disc.
    pub track_count: Option<usize>,
    /// The width (i.e., decimal digits count) of the number of tracks on the disc.
    pub track_count_width: Option<usize>,
    /// The album's title.
    pub album_title: Option<Cow<'a, str>>,
    /// The album's artist (as credited for this release).
    pub album_artist: Option<Cow<'a, str>>,
    /// The album's release date.
    pub date: Option<Cow<'a, str>>,
    /// The album's release year.
    pub year: Option<Cow<'a, str>>,
    /// The disc number.
    pub disc_number: Option<u32>,
    /// The total number of discs that are part of this release.
    pub disc_count: Option<usize>,
    /// The width (i.e., decimal digits count) of the number of discs of this release.
    pub disc_count_width: Option<usize>,
}

impl<'a> PathFormatterValues<'a> {
    /// Assign fields from a [`ReleaseLike`] object.
    pub fn with_release(mut self, release: &'a impl ReleaseLike) -> Self {
        self.album_title = release.release_title();
        self.album_artist = release.release_artist();
        self.date = release.release_date();
        self.year = release.release_year();
        self.disc_count = Some(release.media().count());
        self.disc_count_width = self.disc_count.and_then(usize_width);
        self
    }

    /// Assign fields from a [`MediaLike`] object.
    pub fn with_media(mut self, media: &'a impl MediaLike) -> Self {
        self.disc_number = media.disc_number();
        self.track_count = Some(media.media_tracks().count());
        self.track_count_width = self.track_count.and_then(usize_width);
        self
    }

    /// Assign fields from a [`TrackLike`] object.
    pub fn with_track(mut self, index: usize, track: &'a impl TrackLike) -> Self {
        self.track_title = track.track_title();
        self.track_artist = track.track_artist();
        self.track_number = track.track_number();
        self.track_index = index.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use serde_json::json;

    use musicbrainz_rs_nova::entity::release::Release as MusicBrainzRelease;

    const MUSICBRAINZ_RELEASE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/musicbrainz/release.json"
    ));

    #[test]
    fn test_helper_zfill() {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_helper("zfill", Box::new(helper_zfill));
        handlebars.register_template(
            "test",
            Template::compile("{{zfill index width=width}}").unwrap(),
        );
        assert_eq!(
            handlebars
                .render("test", &json!({ "index": 8, "width": 1}))
                .unwrap(),
            "8".to_string()
        );
        assert_eq!(
            handlebars
                .render("test", &json!({ "index": 8, "width": 2}))
                .unwrap(),
            "08".to_string()
        );
        assert_eq!(
            handlebars
                .render("test", &json!({ "index": 8, "width": 3}))
                .unwrap(),
            "008".to_string()
        );
    }

    #[test]
    fn test_usize_width() {
        for i in 0..=9 {
            assert_eq!(usize_width(i), Some(1));
        }
        assert_eq!(usize_width(10), Some(2));
        assert_eq!(usize_width(50), Some(2));
        assert_eq!(usize_width(99), Some(2));
        assert_eq!(usize_width(100), Some(3));
        assert_eq!(usize_width(250), Some(3));
        assert_eq!(usize_width(512), Some(3));
        assert_eq!(usize_width(768), Some(3));
        assert_eq!(usize_width(999), Some(3));
        assert_eq!(usize_width(1000), Some(4));
        assert_eq!(usize_width(1111), Some(4));
        assert_eq!(usize_width(1250), Some(4));
        assert_eq!(usize_width(4567), Some(4));
        assert_eq!(usize_width(8901), Some(4));
        assert_eq!(usize_width(9999), Some(4));
        assert_eq!(usize_width(10000), Some(5));
    }

    #[test]
    fn test_album_path() {
        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let media = release.media().next().unwrap();
        let track = media.media_tracks().next().unwrap();

        let config = Config::default();
        let formatter = config.paths.format.formatter();
        let values = PathFormatterValues::default()
            .with_release(&release)
            .with_media(media)
            .with_track(1, track);

        let output = formatter.format(&values).unwrap();
        assert_eq!(
            output,
            "The Ahmad Jamal Trio/1958 - Ahmad Jamal at the Pershing: But Not for Me/1-1 - But Not for Me"
        );
    }
}
