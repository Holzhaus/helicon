// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Support for ID3 tags.

use crate::tag::{Tag, TagKey, TagType};
use id3::TagLike;
use std::path::Path;

/// ID3 frame ID.
enum FrameId<'a> {
    /// Text frame.
    Text(&'a str),
    /// Extended Text frame (`TXXX`).
    ExtendedText(&'a str),
}

/// ID3 tag (version 2).
pub struct ID3v2Tag {
    /// The underlying tag data.
    data: id3::Tag,
}

impl ID3v2Tag {
    /// Read the ID3 tag from the path
    pub fn read_from_path(path: impl AsRef<Path>) -> Option<Self> {
        id3::Tag::read_from_path(path)
            .ok()
            .map(|data| ID3v2Tag { data })
    }

    /// Get the ID3 frame for a tag key.
    fn tag_key_to_frame(key: &TagKey) -> Option<FrameId<'_>> {
        match key {
            TagKey::Album => FrameId::Text("TALB").into(),
            TagKey::AlbumArtist => FrameId::Text("TPE2").into(),
            TagKey::Artist => FrameId::Text("TPE1").into(),
            TagKey::MusicBrainzReleaseId => FrameId::ExtendedText("MusicBrainz Album Id").into(),
            _ => None,
        }
    }

    /// Get the content of a text frame as string.
    fn get(&self, frame_id: &str) -> Option<&str> {
        self.data
            .get(frame_id)
            .and_then(|frame| frame.content().text())
    }

    /// Get the content of an extended text frame as string.
    fn get_extended_text(&self, description: &str) -> Option<&str> {
        self.data
            .extended_texts()
            .find(|t| t.description == description)
            .and_then(|t| t.value.strip_suffix('\0'))
    }
}

impl Tag for ID3v2Tag {
    fn tag_type(&self) -> TagType {
        match self.data.version() {
            id3::Version::Id3v22 => TagType::ID3v22,
            id3::Version::Id3v23 => TagType::ID3v23,
            id3::Version::Id3v24 => TagType::ID3v24,
        }
    }

    fn get(&self, key: &TagKey) -> Option<&str> {
        Self::tag_key_to_frame(key).and_then(|frame_id| match frame_id {
            FrameId::Text(value) => self.get(value),
            FrameId::ExtendedText(value) => self.get_extended_text(value),
        })
    }
}
