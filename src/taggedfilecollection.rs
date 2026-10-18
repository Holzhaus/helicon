// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utilities for matching and lookup up albums and tracks.

use crate::analyzer::EbuR128AlbumResult;
use crate::coverart::{CoverArt, CoverInfo};
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
use regex::{Regex, RegexBuilder};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    #[cfg(test)]
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

/// Finds the most common value for a certain tag in an iterator of tagged files.
fn find_most_common_tag_value<'a>(
    tracks: impl Iterator<Item = &'a TaggedFile> + 'a,
    key: &'a TagKey,
) -> Option<MostCommonItem<Cow<'a, str>>> {
    MostCommonItem::find(tracks.filter_map(|tagged_file| tagged_file.first_tag_value(key)))
}

/// Strip leading and trailing whitespace from the `Cow` string.
fn trim_cow(value: Cow<'_, str>) -> Cow<'_, str> {
    match value {
        Cow::Borrowed(s) => Cow::Borrowed(s.trim()),
        Cow::Owned(s) => {
            let trimmed = s.trim();
            if trimmed.len() == s.len() {
                Cow::Owned(s)
            } else {
                Cow::Owned(trimmed.to_owned())
            }
        }
    }
}

/// Strip disc suffix from the end of the `Cow` string.
fn strip_disc_suffix<'a>(title: Cow<'a, str>, pattern: &Regex) -> Cow<'a, str> {
    match pattern.find(&title) {
        Some(m) => {
            let start = m.start();
            match title {
                Cow::Borrowed(s) => Cow::Borrowed(&s[..start]),
                Cow::Owned(mut s) => {
                    s.truncate(start);
                    Cow::Owned(s)
                }
            }
        }
        None => title,
    }
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

/// How track files are transferred into the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    /// Move the files into the library.
    Move,
    /// Copy the files into the library.
    Copy,
    /// Leave the files where they are.
    InPlace,
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

    /// Like `find_consensual_tag_value`, but ignores leading and trailing whitespace.
    fn find_consensual_trimmed_tag_value<'a>(&'a self, key: &'a TagKey) -> Option<Cow<'a, str>> {
        MostCommonItem::find(
            self.media
                .iter()
                .flat_map(|media| media.tracks.iter())
                .filter_map(|tagged_file| tagged_file.first_tag_value(key))
                .map(trim_cow),
        )
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

    /// The deepest directory that contains all tracks of this collection.
    #[must_use]
    pub fn common_directory(&self) -> Option<PathBuf> {
        self.media
            .iter()
            .flat_map(|media| media.tracks.iter())
            .filter_map(|tagged_file| tagged_file.track_path()?.parent().map(Path::to_path_buf))
            .reduce(|common, dir| {
                common
                    .components()
                    .zip(dir.components())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a)
                    .collect()
            })
    }

    /// Move or copy files for all tracks in this collection into the library.
    ///
    /// # Errors
    ///
    /// Returns an error if moving or copying any of the files fails.
    pub fn transfer_files(&mut self, config: &Config, mode: TransferMode) -> crate::Result<()> {
        if mode == TransferMode::InPlace {
            return Ok(());
        }

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
            match mode {
                TransferMode::Move => util::move_file(&track.path, &dest_path)?,
                TransferMode::Copy => util::copy_file(&track.path, &dest_path)?,
                TransferMode::InPlace => unreachable!(),
            }
            track.path = dest_path;
        }

        Ok(())
    }

    /// For each track (in order), information about its existing front cover (if any).
    #[must_use]
    pub fn front_cover_infos(&self) -> Vec<Option<CoverInfo>> {
        self.media
            .iter()
            .flat_map(|media| media.tracks.iter())
            .map(TaggedFile::front_cover_info)
            .collect()
    }

    /// Whether any track would accept a front cover with the given overwrite behavior.
    #[must_use]
    pub fn needs_front_cover(&self, overwrite_existing: bool) -> bool {
        self.media
            .iter()
            .flat_map(|media| media.tracks.iter())
            .any(|track| track.needs_front_cover(overwrite_existing))
    }

    /// Set the front cover for all tracks in this collection.
    ///
    /// Tracks that already have a front cover are skipped unless `overwrite_existing` is set.
    pub fn set_front_cover(&mut self, cover: &CoverArt, overwrite_existing: bool) {
        for track in self
            .media
            .iter_mut()
            .flat_map(|media| media.tracks.iter_mut())
        {
            track.set_front_cover(cover, overwrite_existing);
        }
    }

    /// Remove the front cover from all tracks in this collection.
    pub fn remove_front_cover(&mut self) {
        for track in self
            .media
            .iter_mut()
            .flat_map(|media| media.tracks.iter_mut())
        {
            track.remove_front_cover();
        }
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

    /// Set permission for all tracks in this collection.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the underlying file system operations fail.
    #[cfg(unix)]
    pub fn set_permissions(&mut self, config: &Config) -> crate::Result<()> {
        for track in &mut self
            .media
            .iter_mut()
            .flat_map(|media| media.tracks.iter_mut())
        {
            util::set_file_permissions(
                &track.path,
                config.import.set_uid,
                config.import.set_gid,
                config.import.set_mode,
            )?;
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
        let disc_pattern = RegexBuilder::new(r"\s*[(\[-]\s*(?:CD|Disc)\s*\d+\s*[)\]]?\s*$")
            .case_insensitive(true)
            .build()
            .unwrap();

        MostCommonItem::find(
            self.media
                .iter()
                .flat_map(|media| media.tracks.iter())
                .filter_map(|tagged_file| tagged_file.first_tag_value(&TagKey::Album))
                .map(|title| trim_cow(strip_disc_suffix(trim_cow(title), &disc_pattern))),
        )
        .and_then(MostCommonItem::into_concensus)
    }

    fn release_artist(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_trimmed_tag_value(&TagKey::AlbumArtist)
            .or_else(|| self.find_consensual_trimmed_tag_value(&TagKey::Artist))
    }

    fn release_artist_sort_order(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::AlbumArtistSortOrder)
    }

    fn release_sort_order(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::AlbumSortOrder)
    }

    fn asin(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Asin)
    }

    fn barcode(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Barcode)
    }

    fn catalog_number(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::CatalogNumber)
    }

    fn compilation(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Compilation)
    }

    fn grouping(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Grouping)
    }

    fn musicbrainz_release_artist_id(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::MusicBrainzReleaseArtistId)
    }

    fn musicbrainz_release_group_id(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::MusicBrainzReleaseGroupId)
    }

    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::MusicBrainzReleaseId)
    }

    fn record_label(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::RecordLabel)
    }

    fn release_country(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseCountry)
    }

    fn release_date(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseDate)
    }

    fn release_year(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseYear)
    }

    fn release_status(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseStatus)
    }

    fn release_type(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::ReleaseType)
    }

    fn script(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Script)
    }

    fn total_discs(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::TotalDiscs)
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
    }

    fn media_format(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::Media)
    }

    fn musicbrainz_disc_id(&self) -> Option<Cow<'_, str>> {
        self.find_consensual_tag_value(&TagKey::MusicBrainzDiscId)
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
    fn test_common_directory() {
        let collection_for = |paths: &[&str]| {
            TaggedFileCollection::new(
                paths
                    .iter()
                    .map(|path| {
                        let mut file = TaggedFile::new(vec![]);
                        file.path = PathBuf::from(path);
                        file
                    })
                    .collect(),
            )
        };
        assert_eq!(
            collection_for(&["/music/album/01.mp3", "/music/album/02.mp3"]).common_directory(),
            Some(PathBuf::from("/music/album"))
        );
        assert_eq!(
            collection_for(&["/music/album/CD1/01.mp3", "/music/album/CD2/01.mp3"])
                .common_directory(),
            Some(PathBuf::from("/music/album"))
        );
        assert_eq!(collection_for(&[]).common_directory(), None);
    }

    #[test]
    #[cfg(feature = "id3")]
    fn test_release_title_and_artist_are_trimmed() {
        use crate::tag::id3::ID3v2Tag;

        let values = [
            ("  Album (Disc 1) ", " Artist"),
            ("Album (Disc 2)", "Artist  "),
        ];
        let tracks = values
            .iter()
            .map(|(album, artist)| {
                let mut file =
                    TaggedFile::new(vec![Box::new(ID3v2Tag::with_version(id3::Version::Id3v24))]);
                file.set_tag_value(&TagKey::Album, Some(Cow::from(*album)));
                file.set_tag_value(&TagKey::AlbumArtist, Some(Cow::from(*artist)));
                file
            })
            .collect();
        let collection = TaggedFileCollection::new(tracks);
        assert_eq!(collection.release_title().as_deref(), Some("Album"));
        assert_eq!(collection.release_artist().as_deref(), Some("Artist"));
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
        assert_eq!(collection.release_date().as_deref(), Some("1958"));
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
        assert_eq!(collection.release_date().as_deref(), Some("1958"));
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
        assert_eq!(collection.release_date().as_deref(), Some("1958"));
        assert_eq!(collection.release_year().as_deref(), Some("1958"));
        assert_eq!(collection.release_status().as_deref(), Some("official"));
        assert_eq!(collection.release_type().as_deref(), Some("album"));
        assert_eq!(collection.script().as_deref(), Some("Latn"));
        assert_eq!(collection.total_discs().as_deref(), Some("1"));
    }

    #[test]
    #[cfg(feature = "flac")]
    fn test_set_front_cover_on_collection() {
        use crate::coverart::testing::jpeg_cover;
        use crate::tag::flac::FlacTag;

        let mut collection = make_collection_from_assignment(|| Box::new(FlacTag::new()));
        assert!(collection.needs_front_cover(false));
        let total = collection.front_cover_infos().len();
        assert!(total > 0);
        assert!(collection.front_cover_infos().iter().all(Option::is_none));
        collection.set_front_cover(&jpeg_cover(), false);
        let expected = Some(CoverInfo::from_data(jpeg_cover().data()));
        assert_eq!(collection.front_cover_infos(), vec![expected; total]);
        assert!(!collection.needs_front_cover(false));
        assert!(collection.needs_front_cover(true));
        assert!(collection
            .media
            .iter()
            .flat_map(|media| media.tracks.iter())
            .all(|track| !track.needs_front_cover(false)));
    }
}
