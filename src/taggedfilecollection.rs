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
use itertools::Itertools;
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
        "" | "various artists"
            | "various"
            | "va"
            | "v.a."
            | "[various]"
            | "[various artists]"
            | "unknown"
    )
}

/// Finds the most common value for a certain tag in an iterator of tagged files.
fn find_most_common_tag_value<'a>(
    tracks: impl Iterator<Item = &'a TaggedFile> + 'a,
    key: &'a TagKey,
) -> Option<MostCommonItem<Cow<'a, str>>> {
    MostCommonItem::find(tracks.filter_map(|tagged_file| tagged_file.first_tag_value(key)))
}

/// A collection of tracks on the local disk.
#[derive(Debug)]
pub struct TaggedFileCollection {
    /// List of media in this collection. These are determined by the "disc number" value of each
    /// track.
    media: Vec<TaggedFileMedia>,
    /// EBU R128 Album Result
    ebur128_album_result: Option<EbuR128AlbumResult>,
}

impl TaggedFileCollection {
    /// Creates a new collection from a `Vec` of `TaggedFile` instances.
    #[must_use]
    pub fn new(mut tracks: Vec<TaggedFile>) -> Self {
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

        tracks.sort_by_cached_key(|track| {
            (
                track
                    .first_tag_value(&TagKey::DiscNumber)
                    .map_or_else(String::new, |n| n.to_string()),
                track.path.clone(),
            )
        });
        let media = tracks
            .into_iter()
            .chunk_by(|track| {
                track
                    .first_tag_value(&TagKey::DiscNumber)
                    .map(|x| x.to_string())
            })
            .into_iter()
            .map(|(_key, tracks)| TaggedFileMedia {
                tracks: tracks.collect::<Vec<_>>(),
            })
            .collect();

        Self {
            media,
            ebur128_album_result,
        }
    }

    /// Finds the consensual value for a certain tag in an iterator of tagged files.
    ///
    /// Returns `None` if there is no consensual value.
    fn find_consensual_tag_value<'a>(&'a self, key: &'a TagKey) -> Option<Cow<'a, str>> {
        find_most_common_tag_value(self.media.iter().flat_map(|media| media.tracks.iter()), key)
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
        let tracks = self
            .media
            .into_iter()
            .flat_map(|media| media.tracks.into_iter())
            .enumerate()
            .filter_map(|(i, track)| {
                matched_track_map.get(&i).map(|(j, _)| j).and_then(|j| {
                    Some(track).zip(
                        release_candidate
                            .release()
                            .media()
                            .enumerate()
                            .flat_map(|(media_index, media)| {
                                media
                                    .media_tracks()
                                    .map(move |media_track| (media_index + 1, media, media_track))
                            })
                            .nth(*j),
                    )
                })
            })
            .map(
                move |(mut track, (media_index, other_media, other_track))| {
                    track.assign_tags_from_track(other_track);
                    track.set_tag_value(
                        &TagKey::DiscNumber,
                        Some(Cow::from(format!("{media_index}"))),
                    );
                    track.assign_tags_from_media(other_media);
                    track.assign_tags_from_release(release_candidate.release());
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
                },
            )
            .collect();
        self = TaggedFileCollection::new(tracks);
        self
    }

    /// Move files for all tracks in this collection.
    ///
    /// # Errors
    ///
    /// Returns an error if moving any of the files fails.
    pub fn move_files(&mut self, config: &Config) -> crate::Result<()> {
        let paths = self
            .media()
            .flat_map(|media| media.media_tracks().map(move |track| (media, track)))
            .enumerate()
            .map(|(i, (media, track))| {
                let values = PathFormatterValues::default()
                    .with_release(self)
                    .with_media(media)
                    .with_track(i + 1, track);
                config
                    .paths
                    .format_path(&values, track.track_file_extension())
            })
            .collect::<crate::Result<Vec<_>>>()?;

        for (track, dest_path) in self
            .media
            .iter_mut()
            .flat_map(|media| media.tracks.iter_mut())
            .zip(paths)
        {
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
        for track in &mut self
            .media
            .iter_mut()
            .flat_map(|media| media.tracks.iter_mut())
        {
            track.write_tags()?;
        }

        Ok(())
    }
}

impl IntoIterator for TaggedFileCollection {
    type Item = TaggedFile;
    type IntoIter = std::iter::FlatMap<
        std::vec::IntoIter<TaggedFileMedia>,
        std::vec::IntoIter<TaggedFile>,
        fn(TaggedFileMedia) -> std::vec::IntoIter<Self::Item>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.media
            .into_iter()
            .flat_map(|media| media.tracks.into_iter())
    }
}

impl FromIterator<TaggedFile> for TaggedFileCollection {
    fn from_iter<I: IntoIterator<Item = TaggedFile>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect::<Vec<TaggedFile>>())
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
            .find_map(|key| {
                find_most_common_tag_value(
                    self.media.iter().flat_map(|media| media.tracks.iter()),
                    key,
                )
            })
            .map(MostCommonItem::into_inner)
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
        self.media.iter()
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

    fn is_compilation(&self) -> bool {
        self.release_artist().as_deref().is_some_and(is_va_artist)
            || find_most_common_tag_value(
                self.media.iter().flat_map(|media| media.tracks.iter()),
                &TagKey::Artist,
            )
            .is_some_and(|most_common_artist| most_common_artist.is_all_distinct())
    }
}

/// Media inside a tagged file collection.
#[derive(Debug, PartialEq)]
pub struct TaggedFileMedia {
    /// Tracks on this media.
    tracks: Vec<TaggedFile>,
}

impl TaggedFileMedia {
    /// Finds the consensual value for a certain tag in an iterator of tagged files.
    ///
    /// Returns `None` if there is no consensual value.
    fn find_consensual_tag_value<'a, 'b>(&'a self, key: &'b TagKey) -> Option<Cow<'a, str>>
    where
        'b: 'a,
    {
        find_most_common_tag_value(self.tracks.iter(), key).and_then(MostCommonItem::into_concensus)
    }
}

impl MediaLike for TaggedFileMedia {
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

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::distance::ReleaseSimilarity;
    use crate::musicbrainz::MusicBrainzRelease;
    use crate::tag::Tag;

    const MUSICBRAINZ_RELEASE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/musicbrainz/release.json"
    ));

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

    fn make_collection_from_assignment(func: impl Fn() -> Box<dyn Tag>) -> TaggedFileCollection {
        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let release_track_count = release.release_track_count().unwrap();
        let release_candidate =
            ReleaseCandidate::with_similarity(release, ReleaseSimilarity::new(release_track_count));

        let tracks = (0..release_track_count)
            .map(|_| TaggedFile::new(vec![func()]))
            .collect();
        TaggedFileCollection::new(tracks).assign_tags(&release_candidate)
    }

    #[test]
    #[cfg(feature = "id3")]
    fn test_assign_tags_id3v23() {
        use crate::tag::id3::ID3v2Tag;

        let collection = make_collection_from_assignment(|| {
            Box::new(ID3v2Tag::with_version(id3::Version::Id3v23))
        });
        assert_eq!(collection.release_track_count(), Some(8));
        assert_eq!(
            collection.release_title().as_deref(),
            Some("Ahmad Jamal at the Pershing: But Not for Me")
        );
        assert_eq!(
            collection.release_artist().as_deref(),
            Some("The Ahmad Jamal Trio")
        );
        assert_eq!(
            collection.release_artist_sort_order().as_deref(),
            Some("Jamal, Ahmad, Trio, The")
        );
        assert_eq!(collection.release_sort_order(), None);

        assert_eq!(collection.asin(), None);
        assert_eq!(collection.barcode(), None);
        assert_eq!(collection.catalog_number().as_deref(), Some("LP-628"));
        assert_eq!(collection.compilation(), None);
        assert_eq!(collection.grouping(), None);
        assert_eq!(
            collection.musicbrainz_release_artist_id().as_deref(),
            Some("9e7ca87b-4e3d-4d14-90f1-a74acb645fe2")
        );
        assert_eq!(
            collection.musicbrainz_release_group_id().as_deref(),
            Some("0a8e97fd-457c-30bc-938a-2fba79cb04e7")
        );
        assert_eq!(
            collection.musicbrainz_release_id().as_deref(),
            Some("0008f765-032b-46cd-ab69-2220edab1837")
        );
        assert_eq!(collection.record_label().as_deref(), Some("Argo"));
        assert_eq!(collection.release_country().as_deref(), Some("US"));
        assert_eq!(collection.release_date().as_deref(), Some("1958-01-01"));
        assert_eq!(collection.release_year().as_deref(), Some("1958"));
        assert_eq!(collection.release_status().as_deref(), Some("official"));
        assert_eq!(collection.release_type().as_deref(), Some("album"));
        assert_eq!(collection.script().as_deref(), Some("Latn"));
        assert_eq!(collection.total_discs().as_deref(), Some("1"));
    }

    #[test]
    #[cfg(feature = "id3")]
    fn test_assign_tags_id3v24() {
        use crate::tag::id3::ID3v2Tag;

        let collection = make_collection_from_assignment(|| {
            Box::new(ID3v2Tag::with_version(id3::Version::Id3v24))
        });
        assert_eq!(collection.release_track_count(), Some(8));
        assert_eq!(
            collection.release_title().as_deref(),
            Some("Ahmad Jamal at the Pershing: But Not for Me")
        );
        assert_eq!(
            collection.release_artist().as_deref(),
            Some("The Ahmad Jamal Trio")
        );
        assert_eq!(
            collection.release_artist_sort_order().as_deref(),
            Some("Jamal, Ahmad, Trio, The")
        );
        assert_eq!(collection.release_sort_order(), None);

        assert_eq!(collection.asin(), None);
        assert_eq!(collection.barcode(), None);
        assert_eq!(collection.catalog_number().as_deref(), Some("LP-628"));
        assert_eq!(collection.compilation(), None);
        assert_eq!(collection.grouping(), None);
        assert_eq!(
            collection.musicbrainz_release_artist_id().as_deref(),
            Some("9e7ca87b-4e3d-4d14-90f1-a74acb645fe2")
        );
        assert_eq!(
            collection.musicbrainz_release_group_id().as_deref(),
            Some("0a8e97fd-457c-30bc-938a-2fba79cb04e7")
        );
        assert_eq!(
            collection.musicbrainz_release_id().as_deref(),
            Some("0008f765-032b-46cd-ab69-2220edab1837")
        );
        assert_eq!(collection.record_label().as_deref(), Some("Argo"));
        assert_eq!(collection.release_country().as_deref(), Some("US"));
        assert_eq!(collection.release_date().as_deref(), Some("1958-01-01"));
        assert_eq!(collection.release_year().as_deref(), Some("1958"));
        assert_eq!(collection.release_status().as_deref(), Some("official"));
        assert_eq!(collection.release_type().as_deref(), Some("album"));
        assert_eq!(collection.script().as_deref(), Some("Latn"));
        assert_eq!(collection.total_discs().as_deref(), Some("1"));
    }

    #[test]
    #[cfg(feature = "flac")]
    fn test_assign_tags_flac() {
        use crate::tag::flac::FlacTag;

        let collection = make_collection_from_assignment(|| Box::new(FlacTag::new()));
        assert_eq!(collection.release_track_count(), Some(8));
        assert_eq!(
            collection.release_title().as_deref(),
            Some("Ahmad Jamal at the Pershing: But Not for Me")
        );
        assert_eq!(
            collection.release_artist().as_deref(),
            Some("The Ahmad Jamal Trio")
        );
        assert_eq!(
            collection.release_artist_sort_order().as_deref(),
            Some("Jamal, Ahmad, Trio, The")
        );
        assert_eq!(collection.release_sort_order(), None);

        assert_eq!(collection.asin(), None);
        assert_eq!(collection.barcode(), None);
        assert_eq!(collection.catalog_number().as_deref(), Some("LP-628"));
        assert_eq!(collection.compilation(), None);
        assert_eq!(collection.grouping(), None);
        assert_eq!(
            collection.musicbrainz_release_artist_id().as_deref(),
            Some("9e7ca87b-4e3d-4d14-90f1-a74acb645fe2")
        );
        assert_eq!(
            collection.musicbrainz_release_group_id().as_deref(),
            Some("0a8e97fd-457c-30bc-938a-2fba79cb04e7")
        );
        assert_eq!(
            collection.musicbrainz_release_id().as_deref(),
            Some("0008f765-032b-46cd-ab69-2220edab1837")
        );
        assert_eq!(collection.record_label().as_deref(), Some("Argo"));
        assert_eq!(collection.release_country().as_deref(), Some("US"));
        assert_eq!(collection.release_date().as_deref(), Some("1958-01-01"));
        assert_eq!(collection.release_year().as_deref(), Some("1958"));
        assert_eq!(collection.release_status().as_deref(), Some("official"));
        assert_eq!(collection.release_type().as_deref(), Some("album"));
        assert_eq!(collection.script().as_deref(), Some("Latn"));
        assert_eq!(collection.total_discs().as_deref(), Some("1"));
    }
}
