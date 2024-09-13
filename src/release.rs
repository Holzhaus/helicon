// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Generic release implementations.
use crate::distance::{Distance, DistanceBetween};
use musicbrainz_rs_nova::entity::release::Release as MusicBrainzRelease;
use std::borrow::Cow;

/// Represent a generic release, independent of the underlying source.
pub trait Release {
    /// Number of tracks.
    fn track_count(&self) -> Option<usize>;
    /// Release title.
    fn release_title(&self) -> Option<Cow<'_, str>>;
    /// Release artist.
    fn release_artist(&self) -> Option<Cow<'_, str>>;
    /// MusicBrainz Release ID
    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>>;
    /// Catalog Number
    fn catalog_number(&self) -> Option<Cow<'_, str>>;
    /// Barcode
    fn barcode(&self) -> Option<Cow<'_, str>>;

    /// Calculate the distance between this release and another one.
    fn distance_to<T>(&self, other: &T) -> Distance
    where
        Self: Sized,
        T: Release,
    {
        Distance::between(self, other)
    }
}

impl Release for MusicBrainzRelease {
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
