// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utilities for matching and lookup up albums and tracks.

use crate::analyzer::EbuR128AlbumResult;
use crate::media::MediaLike;
use crate::pathformat::PathFormatterValues;
use crate::release::ReleaseLike;
use crate::release_candidate::ReleaseCandidate;
use crate::tag::TagKey;
use crate::track::TrackLike;
use crate::util;
use crate::Config;
use crate::TaggedFile;
use std::borrow::Cow;
use std::collections::HashMap;

/// Represents the the count of a specific item and the first index at which that item was found.
///
/// Used internally by the [`MostCommonItem`] implementation.
struct ItemCounter {
    /// The first index at which the item was found.
    pub first_index: usize,
    /// The count of the item.
    pub count: usize,
}

impl ItemCounter {
    /// Create a new [`ItemCounter`] with count set to zero and the given index.
    fn new_at_index(first_index: usize) -> Self {
        Self {
            first_index,
            count: 0,
        }
    }

    /// Increases the counter by one.
    fn increase_count_by_one(&mut self) {
        self.count += 1;
    }
}

/// Utility struct returned by the [`MostCommonItem::find`] function.
#[derive(Debug, Clone, PartialEq)]
struct MostCommonItem<T> {
    /// The most common item.
    item: T,
    /// The count of the most common item.
    count: usize,
    /// The total number of items.
    total_items_count: usize,
}

impl<T> MostCommonItem<T> {
    /// Finds the most common item in the iterator.
    ///
    /// The function returns `None` if the iterator is empty. In all other cases, it returns the most
    /// common item, its count, the number of distinct items and and the total number of items. If
    /// there are multiple items with an equal count, the first one is returned.
    fn find<I>(items: I) -> Option<Self>
    where
        I: Iterator<Item = T>,
        T: Eq + std::hash::Hash,
    {
        let mut counts = HashMap::new();
        items.enumerate().for_each(|(index, item)| {
            counts
                .entry(item)
                .or_insert_with(|| ItemCounter::new_at_index(index))
                .increase_count_by_one();
        });
        let total_items_count = counts.values().map(|counter| counter.count).sum();
        counts
            .into_iter()
            .max_by_key(|(_, counter)| (counter.count, usize::MAX - counter.first_index))
            .map(|(item, counter)| MostCommonItem {
                item,
                count: counter.count,
                total_items_count,
            })
    }

    /// Return `true` if there a multiple values which are all distinct, otherwise `false`
    fn is_all_distinct(&self) -> bool {
        self.count == 1 && self.total_items_count > 1
    }

    /// Return `true` if the the item is concensual, otherwise `false`
    fn is_concensual(&self) -> bool {
        self.count == self.total_items_count
    }

    /// Return `Some(value)` if the item is consensual, otherwise `None`
    fn into_inner(self) -> T {
        self.item
    }

    /// Return `Some(value)` if the item is consensual, otherwise `None`
    fn into_concensus(self) -> Option<T> {
        self.is_concensual().then_some(self.into_inner())
    }
}

/// Returns `true` if the artist is likely "Various Artists".
fn is_va_artist(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "" | "various artists" | "various" | "va" | "unknown"
    )
}

/// A collection of tracks on the local disk.
#[derive(Debug)]
pub struct TaggedFileCollection {
    /// List of tracks in this collection.
    tracks: Vec<TaggedFile>,
    /// EBU R128 Album Result
    ebur128_album_result: Option<EbuR128AlbumResult>,
}

impl TaggedFileCollection {
    /// Creates a new collection from a `Vec` of `TaggedFile` instances.
    #[must_use]
    pub fn new(tracks: Vec<TaggedFile>) -> Self {
        let ebur128_album_result = tracks
            .iter()
            .map(|track| {
                track
                    .analysis_results
                    .as_ref()
                    .and_then(|analysis_result| analysis_result.ebur128.as_ref())
            })
            .map(|opt| opt.and_then(|res| res.as_ref().ok()))
            .collect::<Option<Vec<_>>>()
            .and_then(|ebur128_results| EbuR128AlbumResult::from_iter(ebur128_results.into_iter()));
        Self {
            tracks,
            ebur128_album_result,
        }
    }

    /// Finds the most common value for a certain tag in an iterator of tagged files.
    fn find_most_common_tag_value<'a>(
        &'a self,
        key: &'a TagKey,
    ) -> Option<MostCommonItem<Cow<'a, str>>> {
        MostCommonItem::find(
            self.tracks
                .iter()
                .filter_map(|tagged_file| tagged_file.first_tag_value(key)),
        )
    }

    /// Finds the consensual value for a certain tag in an iterator of tagged files.
    ///
    /// Returns `None` if there is no consensual value.
    fn find_consensual_tag_value<'a>(&'a self, key: &'a TagKey) -> Option<Cow<'a, str>> {
        self.find_most_common_tag_value(key)
            .and_then(MostCommonItem::into_concensus)
    }

    /// Assign tracks from a release candidate.
    #[must_use]
    pub fn assign_tags<T: ReleaseLike>(mut self, release_candidate: &ReleaseCandidate<T>) -> Self {
        let matched_track_map = release_candidate
            .similarity()
            .track_assignment()
            .map_lhs_indices_to_rhs();
        let album_gain_analyzed = self
            .replay_gain_album_gain_analyzed()
            .map(|value| value.to_string());
        let album_peak_analyzed = self
            .replay_gain_album_peak_analyzed()
            .map(|value| value.to_string());
        let album_range_analyzed = self
            .replay_gain_album_range_analyzed()
            .map(|value| value.to_string());
        self.tracks = self
            .tracks
            .into_iter()
            .enumerate()
            .filter_map(|(i, track)| {
                matched_track_map.get(&i).map(|(j, _)| j).and_then(|j| {
                    Some(track).zip(release_candidate.release().release_tracks().nth(*j))
                })
            })
            .map(move |(mut track, other_track)| {
                track.assign_tags_from_release(release_candidate.release());
                track.assign_tags_from_track(other_track);
                track.set_tag_value(
                    &TagKey::ReplayGainAlbumGain,
                    album_gain_analyzed.as_ref().map(Cow::from),
                );
                track.set_tag_value(
                    &TagKey::ReplayGainAlbumPeak,
                    album_peak_analyzed.as_ref().map(Cow::from),
                );
                track.set_tag_value(
                    &TagKey::ReplayGainAlbumRange,
                    album_range_analyzed.as_ref().map(Cow::from),
                );
                track
            })
            .collect();
        self
    }

    /// Move files for all tracks in this collection.
    ///
    /// # Errors
    ///
    /// Returns an error if moving any of the files fails.
    pub fn move_files(&mut self, config: &Config) -> crate::Result<()> {
        let paths = self
            .tracks
            .iter()
            .enumerate()
            .map(|(i, track)| {
                let values = PathFormatterValues::default()
                    .with_release(self)
                    .with_media(self)
                    .with_track(i + 1, track);
                config
                    .paths
                    .format_path(&values, track.track_file_extension())
            })
            .collect::<crate::Result<Vec<_>>>()?;

        for (track, dest_path) in self.tracks.iter_mut().zip(paths) {
            util::move_file(&track.path, &dest_path)?;
            track.path = dest_path;
        }

        Ok(())
    }

    /// Write tags for all tracks in this collection.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the underlying tags fail to write.
    pub fn write_tags(&mut self) -> crate::Result<()> {
        for track in &mut self.tracks {
            track.write_tags()?;
        }

        Ok(())
    }
}

impl IntoIterator for TaggedFileCollection {
    type Item = TaggedFile;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.tracks.into_iter()
    }
}

impl FromIterator<TaggedFile> for TaggedFileCollection {
    fn from_iter<I: IntoIterator<Item = TaggedFile>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect::<Vec<TaggedFile>>())
    }
}

impl MediaLike for TaggedFileCollection {
    fn disc_number(&self) -> Option<u32> {
        self.find_consensual_tag_value(&TagKey::DiscNumber)
            .and_then(|number| number.parse::<u32>().ok())
    }

    fn media_title(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::DiscSubtitle)
            .map(Cow::from)
    }

    fn media_format(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Media)
            .map(Cow::from)
    }

    fn musicbrainz_disc_id(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::MusicBrainzDiscId)
            .map(Cow::from)
    }

    fn media_track_count(&self) -> Option<usize> {
        self.tracks.len().into()
    }

    fn gapless_playback(&self) -> Option<bool> {
        self.find_consensual_tag_value(&TagKey::GaplessPlayback)
            .map(|value| {
                let value_lower = value.to_ascii_lowercase();
                let result = &["1", "on", "true", "yes"].contains(&value_lower.as_str());
                *result
            })
    }

    fn media_tracks(&self) -> impl Iterator<Item = &(impl TrackLike + '_)> {
        self.tracks.iter()
    }
}

impl ReleaseLike for TaggedFileCollection {
    fn release_title(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Album)
            .map(Cow::from)
    }

    fn release_artist(&self) -> Option<Cow<'_, str>> {
        [&TagKey::AlbumArtist, &TagKey::Artist]
            .into_iter()
            .find_map(|key| self.find_most_common_tag_value(key))
            .and_then(|most_common_artist| {
                most_common_artist
                    .is_all_distinct()
                    .then_some(Cow::Borrowed("Various Artists"))
                    .or_else(|| {
                        let artist_name = most_common_artist.into_inner();
                        if is_va_artist(&artist_name) {
                            Cow::Borrowed("Various Artists")
                        } else {
                            artist_name
                        }
                        .into()
                    })
            })
    }

    fn release_artist_sort_order(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::AlbumArtistSortOrder)
            .map(Cow::from)
    }

    fn release_sort_order(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::AlbumSortOrder)
            .map(Cow::from)
    }

    fn asin(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Asin).map(Cow::from)
    }

    fn barcode(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Barcode)
            .map(Cow::from)
    }
    fn catalog_number(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::CatalogNumber)
            .map(Cow::from)
    }

    fn compilation(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Compilation)
            .map(Cow::from)
    }

    fn grouping(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Grouping)
            .map(Cow::from)
    }

    fn musicbrainz_release_artist_id(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::MusicBrainzReleaseArtistId)
            .map(Cow::from)
    }

    fn musicbrainz_release_group_id(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::MusicBrainzReleaseGroupId)
            .map(Cow::from)
    }

    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::MusicBrainzReleaseId)
            .map(Cow::from)
    }
    fn record_label(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::RecordLabel)
            .map(Cow::from)
    }
    fn release_country(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseCountry)
            .map(Cow::from)
    }
    fn release_date(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseDate)
            .map(Cow::from)
    }

    fn release_year(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseYear)
            .map(Cow::from)
    }

    fn release_status(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseStatus)
            .map(Cow::from)
    }

    fn release_type(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseType)
            .map(Cow::from)
    }

    fn script(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Script)
            .map(Cow::from)
    }

    fn total_discs(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::TotalDiscs)
            .map(Cow::from)
    }

    fn media(&self) -> impl Iterator<Item = &(impl MediaLike + '_)> {
        std::iter::once(self)
    }

    fn replay_gain_album_gain_analyzed(&self) -> Option<Cow<'_, str>> {
        self.ebur128_album_result
            .as_ref()
            .map(|result| Cow::from(result.replaygain_album_gain_string()))
    }

    fn replay_gain_album_peak_analyzed(&self) -> Option<Cow<'_, str>> {
        self.ebur128_album_result
            .as_ref()
            .map(|result| Cow::from(result.replaygain_album_peak_string()))
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_most_common_item_with_empty_string_iterator() {
        assert_eq!(None, MostCommonItem::find(std::iter::empty::<String>()));
    }

    #[test]
    fn test_most_common_item_with_str_iterator() {
        let values = ["frog", "dog", "horse", "dog", "cat", "cat", "dog", "mouse"];
        let most_common_item =
            MostCommonItem::find(values.iter()).expect("No most common item found!");
        assert_eq!(
            MostCommonItem {
                item: &"dog",
                count: 3,
                total_items_count: 8
            },
            most_common_item
        );
        assert!(!most_common_item.is_concensual());
        assert!(!most_common_item.is_all_distinct());
        assert_eq!(&"dog", most_common_item.clone().into_inner());
        assert_eq!(None, most_common_item.into_concensus());
    }

    #[test]
    fn test_most_common_item_with_str_iterator_concensus() {
        let values = ["dog", "dog", "dog"];
        let most_common_item =
            MostCommonItem::find(values.iter()).expect("No most common item found!");
        assert_eq!(
            MostCommonItem {
                item: &"dog",
                count: 3,
                total_items_count: 3
            },
            most_common_item
        );
        assert!(most_common_item.is_concensual());
        assert!(!most_common_item.is_all_distinct());
        assert_eq!(&"dog", most_common_item.clone().into_inner());
        assert_eq!(Some(&"dog"), most_common_item.into_concensus());
    }

    #[test]
    fn test_most_common_item_with_str_iterator_distinct() {
        let values = ["frog", "dog", "horse", "cat", "mouse"];
        let most_common_item =
            MostCommonItem::find(values.iter()).expect("No most common item found!");
        assert_eq!(
            MostCommonItem {
                item: &"frog",
                count: 1,
                total_items_count: 5
            },
            most_common_item
        );
        assert!(!most_common_item.is_concensual());
        assert!(most_common_item.is_all_distinct());
        assert_eq!(&"frog", most_common_item.clone().into_inner());
        assert_eq!(None, most_common_item.into_concensus());
    }

    #[test]
    fn test_most_common_item_find_with_int_iterator() {
        let values = [1, 2, 3, 4, 5, 1, 2, 3, 2, 5, 9, 8, 2];
        let most_common_item =
            MostCommonItem::find(values.iter()).expect("No most common item found!");
        assert_eq!(
            MostCommonItem {
                item: &2,
                count: 4,
                total_items_count: 13
            },
            most_common_item
        );
        assert!(!most_common_item.is_concensual());
        assert!(!most_common_item.is_all_distinct());
        assert_eq!(&2, most_common_item.clone().into_inner());
        assert_eq!(None, most_common_item.into_concensus());
    }
}
