// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Support for FLAC tags.

use crate::tag::{Tag, TagKey, TagType};
use std::path::Path;

/// FLAC tag.
pub struct FlacTag {
    /// The underlying tag data.
    data: metaflac::Tag,
}

impl FlacTag {
    /// Read the FLAC tag from the path
    pub fn read_from_path(path: impl AsRef<Path>) -> Option<Self> {
        metaflac::Tag::read_from_path(path)
            .ok()
            .map(|data| FlacTag { data })
    }

    /// Get the vorbis key name for a tag key.
    fn tag_key_to_frame(key: &TagKey) -> Option<&'static str> {
        match key {
            TagKey::Album => "ALBUM".into(),
            TagKey::AlbumArtist => "ALBUMARTIST".into(),
            TagKey::Artist => "ARTIST".into(),
            TagKey::MusicBrainzAlbumId => "MUSICBRAINZ_ALBUMID".into(),
            _ => None,
        }
    }
}

impl Tag for FlacTag {
    fn tag_type(&self) -> TagType {
        TagType::Flac
    }

    fn get(&self, key: &TagKey) -> Option<&str> {
        Self::tag_key_to_frame(key)
            .and_then(|key| self.data.get_vorbis(key))
            .and_then(|mut iterator| iterator.next())
    }
}
