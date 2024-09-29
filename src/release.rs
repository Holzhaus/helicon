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

    /// Title of the release.
    fn release_title(&self) -> Option<Cow<'_, str>>;

    /// Artist(s) primarily credited on the release.
    fn release_artist(&self) -> Option<Cow<'_, str>>;

    /// Release Artist’s Sort Name (e.g.: “Beatles, The”).
    fn release_artist_sort_order(&self) -> Option<Cow<'_, str>>;

    /// Release Title’s Sort Name.
    fn release_sort_order(&self) -> Option<Cow<'_, str>>;

    /// Amazon Standard Identification Number - the number identifying the item on Amazon.
    fn asin(&self) -> Option<Cow<'_, str>>;

    /// Release Barcode - the barcode assigned to the release.
    fn barcode(&self) -> Option<Cow<'_, str>>;

    /// The number(s) assigned to the release by the label(s), which can often be found on the
    /// spine or near the barcode. There may be more than one, especially when multiple labels are
    /// involved.
    fn catalog_number(&self) -> Option<Cow<'_, str>>;

    /// 1 for Various Artist albums, otherwise 0 (compatible with iTunes).
    fn compilation(&self) -> Option<Cow<'_, str>>;

    /// Content Group.
    fn grouping(&self) -> Option<Cow<'_, str>>;

    /// Release Artist’s MusicBrainz Identifier.
    fn musicbrainz_release_artist_id(&self) -> Option<Cow<'_, str>>;

    /// Release Group’s MusicBrainz Identifier.
    fn musicbrainz_release_group_id(&self) -> Option<Cow<'_, str>>;

    /// Release MusicBrainz Identifier.
    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>>;

    /// Release Record Label Name(s).
    fn record_label(&self) -> Option<Cow<'_, str>>;

    /// Country in which the release was issued.
    fn release_country(&self) -> Option<Cow<'_, str>>;

    /// Release Date (YYYY-MM-DD) - the date that the release was issued.
    fn release_date(&self) -> Option<Cow<'_, str>>;

    /// Release Year (YYYY) - the year that the release was issued.
    fn release_year(&self) -> Option<Cow<'_, str>>;

    /// Release Status indicating the “official” status of the release.
    fn release_status(&self) -> Option<Cow<'_, str>>;

    /// Release Group Type.
    fn release_type(&self) -> Option<Cow<'_, str>>;

    /// The script used to write the release’s track list.
    ///
    /// The values should be taken from the ISO 15924 standard.
    fn script(&self) -> Option<Cow<'_, str>>;

    /// Total number of discs in this release.
    fn total_discs(&self) -> Option<Cow<'_, str>>;

    /// MusicBrainz Release URL
    fn musicbrainz_release_url(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_release_id()
            .map(|id| format!("https://musicbrainz.org/release/{id}").into())
    }

    /// Yields the media contained in the release.
    fn media(&self) -> impl Iterator<Item = &(impl MediaLike + '_)>;

    /// Find media formats for a release as a human-readable string.
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

    fn release_artist_sort_order(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn release_sort_order(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn asin(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn barcode(&self) -> Option<Cow<'_, str>> {
        self.barcode.as_deref().map(Cow::from)
    }

    fn catalog_number(&self) -> Option<Cow<'_, str>> {
        self.label_info.as_ref().and_then(|label_infos| {
            label_infos
                .iter()
                .find_map(|label_info| label_info.catalog_number.as_deref())
                .map(Cow::from)
        })
    }

    fn compilation(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn grouping(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn musicbrainz_release_artist_id(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn musicbrainz_release_group_id(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>> {
        Cow::from(self.id.as_str()).into()
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

    fn release_country(&self) -> Option<Cow<'_, str>> {
        self.country.as_ref().map(Cow::from)
    }

    fn release_date(&self) -> Option<Cow<'_, str>> {
        self.date
            .map(|date| date.format("%Y-%m-%d").to_string())
            .map(Cow::from)
    }

    fn release_year(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn release_status(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn release_type(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn script(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }

    fn total_discs(&self) -> Option<Cow<'_, str>> {
        // TODO:: Implement this.
        None
    }
}
