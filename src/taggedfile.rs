// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! The [`TaggedFile`] struct represents a file that contains tags.

use crate::tag::{read_tags_from_path, Tag, TagKey};
use crate::track::TrackLike;
use std::borrow::Cow;
use std::fmt;
use std::path::Path;

/// A tagged file that contains zero or more tags.
pub struct TaggedFile {
    /// Tags that are present in the file.
    content: Vec<Box<dyn Tag>>,
}

impl fmt::Debug for TaggedFile {
    #[expect(unused_results)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let mut s = f.debug_tuple("TaggedFile");
        for tag in self.tags() {
            s.field(&tag.tag_type());
        }
        s.finish()
    }
}

impl TaggedFile {
    #[cfg(test)]
    #[must_use]
    pub fn new(content: Vec<Box<dyn Tag>>) -> Self {
        TaggedFile { content }
    }

    /// Creates a [`TaggedFile`] from the path.
    ///
    /// # Errors
    ///
    /// Returns an error in case the file at the given path does not exist or is unsupported.
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        read_tags_from_path(path).map(|content| Self { content })
    }

    /// Returns zero or more [`Tag`] objects.
    #[must_use]
    pub fn tags(&self) -> &[Box<dyn Tag>] {
        &self.content
    }

    /// Yields all values for the given [`TagKey`].
    pub fn tag_values(&self, key: TagKey) -> impl Iterator<Item = &str> {
        self.tags().iter().filter_map(move |tag| tag.get(key))
    }

    /// Returns the first value for the given [`TagKey`].
    #[must_use]
    pub fn first_tag_value(&self, key: TagKey) -> Option<&str> {
        self.tag_values(key).next()
    }

    /// Assign metadata from another `TrackLike` struct (e.g. a MusicBrainz track).
    pub fn copy_metadata(&mut self, other: &impl TrackLike) {
        for tag in &mut self.content {
            tag.set_or_clear(TagKey::TrackTitle, other.track_title());
            tag.set_or_clear(TagKey::Artist, other.track_artist());
            tag.set_or_clear(TagKey::TrackNumber, other.track_number());
            tag.set_or_clear(
                TagKey::MusicBrainzRecordingId,
                other.musicbrainz_recording_id(),
            );
        }
    }
}

impl TrackLike for TaggedFile {
    fn track_title(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::TrackTitle).map(Cow::from)
    }

    fn track_artist(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Artist)
            .or(self.first_tag_value(TagKey::AlbumArtist))
            .map(Cow::from)
    }

    fn track_number(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::TrackNumber).map(Cow::from)
    }

    fn track_length(&self) -> Option<chrono::TimeDelta> {
        // TODO: Implement track length detection.
        None
    }

    fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicBrainzRecordingId)
            .map(Cow::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musicbrainz_rs_nova::entity::release::{
        Release as MusicBrainzRelease, Track as MusicBrainzTrack,
    };

    const MUSICBRAINZ_RELEASE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/musicbrainz/release.json"
    ));

    #[cfg(feature = "id3")]
    #[test]
    fn test_copy_metadata_id3() {
        use crate::tag::id3::ID3v2Tag;

        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let track: &MusicBrainzTrack =
            &release.media.as_ref().unwrap()[0].tracks.as_ref().unwrap()[0];

        let mut tagged_file = TaggedFile::new(vec![Box::new(ID3v2Tag::new())]);
        assert!(tagged_file.track_title().is_none());
        assert!(tagged_file.track_artist().is_none());
        assert!(tagged_file.track_number().is_none());
        assert!(tagged_file.musicbrainz_recording_id().is_none());

        tagged_file.copy_metadata(track);

        assert!(tagged_file.track_title().is_some());
        assert!(tagged_file.track_artist().is_some());
        assert!(tagged_file.track_number().is_some());
        assert!(tagged_file.musicbrainz_recording_id().is_some());
    }

    #[cfg(feature = "flac")]
    #[test]
    fn test_copy_metadata_flac() {
        use crate::tag::flac::FlacTag;

        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let track: &MusicBrainzTrack =
            &release.media.as_ref().unwrap()[0].tracks.as_ref().unwrap()[0];

        let mut tagged_file = TaggedFile::new(vec![Box::new(FlacTag::new())]);
        assert!(tagged_file.track_title().is_none());
        assert!(tagged_file.track_artist().is_none());
        assert!(tagged_file.track_number().is_none());
        assert!(tagged_file.musicbrainz_recording_id().is_none());

        tagged_file.copy_metadata(track);

        assert!(tagged_file.track_title().is_some());
        assert!(tagged_file.track_artist().is_some());
        assert!(tagged_file.track_number().is_some());
        assert!(tagged_file.musicbrainz_recording_id().is_some());
    }
}
