// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Generic release implementations.
use crate::distance::ReleaseSimilarity;
use crate::media::MediaLike;
use crate::track::TrackLike;
use crate::Config;
use itertools::Itertools;
use musicbrainz_rs_nova::entity::release::Release as MusicBrainzRelease;
use std::borrow::Cow;

/// Represent a generic release, independent of the underlying source.
pub trait ReleaseLike {
    /// Number of tracks.
    fn release_track_count(&self) -> Option<usize> {
        self.media()
            .filter_map(MediaLike::media_track_count)
            .sum::<usize>()
            .into()
    }

    /// Release title.
    fn release_title(&self) -> Option<Cow<'_, str>>;
    /// Release artist.
    fn release_artist(&self) -> Option<Cow<'_, str>>;
    /// MusicBrainz Release ID
    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>>;
    /// MusicBrainz Release URL
    fn musicbrainz_release_url(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_release_id()
            .map(|id| format!("https://musicbrainz.org/release/{id}").into())
    }
    /// Release Date.
    fn release_date(&self) -> Option<Cow<'_, str>>;
    /// Release Country.
    fn release_country(&self) -> Option<Cow<'_, str>>;
    /// Media format
    fn release_media_format(&self) -> Option<Cow<'_, str>> {
        let formats = self
            .media()
            .filter_map(MediaLike::media_format)
            .chunk_by(|format: &Cow<'_, str>| format.to_string())
            .into_iter()
            .map(|(key, group)| (group.count(), key))
            .fold(String::new(), |acc, (count, format)| {
                let counted_format = if count > 1 {
                    format!("{count}×{format}")
                } else {
                    format
                };
                if acc.is_empty() {
                    counted_format
                } else {
                    format!("{acc}+{counted_format}")
                }
            });

        if formats.is_empty() {
            None
        } else {
            Some(Cow::from(formats))
        }
    }
    /// Record Label
    fn record_label(&self) -> Option<Cow<'_, str>>;
    /// Catalog Number
    fn catalog_number(&self) -> Option<Cow<'_, str>>;
    /// Barcode
    fn barcode(&self) -> Option<Cow<'_, str>>;

    /// Yields the media contained in the release.
    fn media(&self) -> impl Iterator<Item = &(impl MediaLike + '_)>;

    /// Yields the tracks contained in the release.
    fn release_tracks(&self) -> impl Iterator<Item = &(impl TrackLike + '_)> {
        self.media().flat_map(MediaLike::media_tracks)
    }

    /// Calculate the distance between this release and another one.
    fn similarity_to<T>(&self, other: &T, config: &Config) -> ReleaseSimilarity
    where
        Self: Sized,
        T: ReleaseLike,
    {
        ReleaseSimilarity::detect(config, self, other)
    }
}

impl ReleaseLike for MusicBrainzRelease {
    fn media(&self) -> impl Iterator<Item = &(impl MediaLike + '_)> {
        self.media.iter().flat_map(|vec| vec.iter())
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

    fn release_date(&self) -> Option<Cow<'_, str>> {
        self.date
            .map(|date| date.format("%Y-%m-%d").to_string())
            .map(Cow::from)
    }

    fn release_country(&self) -> Option<Cow<'_, str>> {
        self.country.as_ref().map(Cow::from)
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
