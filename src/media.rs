// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Release media.

use crate::track::TrackLike;
use musicbrainz_rs_nova::entity::release::Media as MusicBrainzReleaseMedia;
use std::borrow::Cow;

/// Represent a generic release, independent of the underlying source.
pub trait MediaLike {
    /// Disc number.
    fn disc_number(&self) -> Option<u32>;

    /// Media title.
    fn media_title(&self) -> Option<Cow<'_, str>>;

    /// Media format.
    fn media_format(&self) -> Option<Cow<'_, str>>;

    /// MusicBrainz Disc ID.
    fn musicbrainz_disc_id(&self) -> Option<Cow<'_, str>>;

    /// Number of tracks.
    fn media_track_count(&self) -> Option<usize>;

    /// Yields the tracks on the media.
    fn media_tracks(&self) -> impl Iterator<Item = &(impl TrackLike + '_)>;

    /// Indicated if the playback is gapless.
    fn gapless_playback(&self) -> Option<bool>;
}

impl MediaLike for MusicBrainzReleaseMedia {
    fn disc_number(&self) -> Option<u32> {
        self.position
    }

    fn media_title(&self) -> Option<Cow<'_, str>> {
        self.title.as_ref().map(Cow::from)
    }

    fn media_format(&self) -> Option<Cow<'_, str>> {
        self.format.as_ref().map(Cow::from)
    }

    fn musicbrainz_disc_id(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn media_track_count(&self) -> Option<usize> {
        usize::try_from(self.track_count).ok()
    }

    fn gapless_playback(&self) -> Option<bool> {
        None
    }

    fn media_tracks(&self) -> impl Iterator<Item = &(impl TrackLike + '_)> {
        self.tracks.iter().flat_map(|vec| vec.iter())
    }
}
