// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Generic release implementations.
use crate::distance::Distance;
use crate::track::TrackLike;
use crate::Config;
use musicbrainz_rs_nova::entity::release::Release as MusicBrainzRelease;
use std::borrow::Cow;

/// Represent a generic release, independent of the underlying source.
pub trait ReleaseLike {
    /// Number of tracks.
    fn track_count(&self) -> Option<usize>;
    /// Release title.
    fn release_title(&self) -> Option<Cow<'_, str>>;
    /// Release artist.
    fn release_artist(&self) -> Option<Cow<'_, str>>;
    /// MusicBrainz Release ID
    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>>;
    /// Media format
    fn media_format(&self) -> Option<Cow<'_, str>>;
    /// Record Label
    fn record_label(&self) -> Option<Cow<'_, str>>;
    /// Catalog Number
    fn catalog_number(&self) -> Option<Cow<'_, str>>;
    /// Barcode
    fn barcode(&self) -> Option<Cow<'_, str>>;

    /// Yields the tracks contained in the release.
    fn tracks(&self) -> impl Iterator<Item = &(impl TrackLike + '_)>;

    /// Calculate the distance between this release and another one.
    fn distance_to<T>(&self, other: &T, config: &Config) -> Distance
    where
        Self: Sized,
        T: ReleaseLike,
    {
        Distance::between_releases(config, self, other)
    }
}

impl ReleaseLike for MusicBrainzRelease {
    fn track_count(&self) -> Option<usize> {
        self.media
            .as_ref()
            .map(|media_list| {
                media_list
                    .iter()
                    .map(|media| media.track_count)
                    .sum::<u32>()
            })
            .and_then(|track_count| usize::try_from(track_count).ok())
    }

    fn tracks(&self) -> impl Iterator<Item = &(impl TrackLike + '_)> {
        self.media
            .iter()
            .flat_map(|vec| vec.iter())
            .flat_map(|media| media.tracks.iter())
            .flat_map(|vec| vec.iter())
    }

    fn release_title(&self) -> Option<Cow<'_, str>> {
        Cow::from(self.title.as_str()).into()
    }

    fn release_artist(&self) -> Option<Cow<'_, str>> {
        Cow::from(
            self.artist_credit
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

    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>> {
        Cow::from(self.id.as_str()).into()
    }

    fn media_format(&self) -> Option<Cow<'_, str>> {
        self.media
            .iter()
            .flat_map(|media| media.iter())
            .find_map(|media| media.format.as_deref())
            .map(Cow::from)
    }

    fn record_label(&self) -> Option<Cow<'_, str>> {
        self.label_info.as_ref().and_then(|label_infos| {
            label_infos
                .iter()
                .find_map(|label_info| label_info.label.as_ref())
                .map(|label| &label.name)
                .map(Cow::from)
        })
    }

    fn catalog_number(&self) -> Option<Cow<'_, str>> {
        self.label_info.as_ref().and_then(|label_infos| {
            label_infos
                .iter()
                .find_map(|label_info| label_info.catalog_number.as_deref())
                .map(Cow::from)
        })
    }

    fn barcode(&self) -> Option<Cow<'_, str>> {
        self.barcode.as_deref().map(Cow::from)
    }
}
