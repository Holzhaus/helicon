// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Generic release implementations.
use crate::distance::TrackSimilarity;
use crate::Config;
use musicbrainz_rs_nova::entity::release::Track as MusicBrainzReleaseTrack;
use std::borrow::Cow;

/// Represent a generic release, independent of the underlying source.
pub trait TrackLike {
    /// Track title.
    fn track_title(&self) -> Option<Cow<'_, str>>;
    /// Track artist.
    fn track_artist(&self) -> Option<Cow<'_, str>>;
    /// Track number.
    fn track_number(&self) -> Option<Cow<'_, str>>;
    /// Track length.
    fn track_length(&self) -> Option<chrono::TimeDelta>;
    /// MusicBrainz Recording ID
    fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>>;

    /// Calculate the distance between this track and another one.
    fn similarity_to<T>(&self, other: &T, config: &Config) -> TrackSimilarity
    where
        Self: Sized,
        T: TrackLike,
    {
        TrackSimilarity::detect(config, self, other)
    }
}

impl TrackLike for MusicBrainzReleaseTrack {
    fn track_title(&self) -> Option<Cow<'_, str>> {
        Cow::from(self.title.as_str()).into()
    }

    fn track_artist(&self) -> Option<Cow<'_, str>> {
        Cow::from(
            self.recording
                .artist_credit
                .iter()
                .flat_map(|artists| artists.iter())
                .fold(String::new(), |acc, artist| {
                    acc + &artist.name
                        + if let Some(joinphrase) = &artist.joinphrase {
                            joinphrase
                        } else {
                            ""
                        }
                }),
        )
        .into()
    }

    fn track_number(&self) -> Option<Cow<'_, str>> {
        Cow::from(&self.number).into()
    }

    fn track_length(&self) -> Option<chrono::TimeDelta> {
        self.length
            .map(|length| chrono::TimeDelta::milliseconds(length.into()))
    }

    fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>> {
        Cow::from(self.recording.id.as_str()).into()
    }
}
