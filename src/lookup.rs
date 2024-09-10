// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utilities for matching and lookup up albums and tracks.

use crate::release::Release;
use crate::tag::{TagKey, TaggedFile};
use futures::{
    future::{self, FutureExt},
    stream::{self, StreamExt},
    Stream,
};
use musicbrainz_rs_nova::{
    entity::release::{
        Release as MusicBrainzRelease, ReleaseSearchQuery as MusicBrainzReleaseSearchQuery,
    },
    Fetch, Search,
};
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
pub struct TrackCollection(Vec<TaggedFile>);

impl TrackCollection {
    /// Creates a new collection from a `Vec` of `TaggedFile` instances.
    pub fn new(tracks: Vec<TaggedFile>) -> Self {
        Self(tracks)
    }

    /// Finds the most common value for a certain tag in an iterator of tagged files.
    fn find_most_common_tag_value(&self, key: TagKey) -> Option<MostCommonItem<&str>> {
        MostCommonItem::find(
            self.0
                .iter()
                .filter_map(|tagged_file| tagged_file.tags().iter().find_map(|tag| tag.get(key))),
        )
    }

    /// Finds the consensual value for a certain tag in an iterator of tagged files.
    ///
    /// Returns `None` if there is no consensual value.
    fn find_consensual_tag_value(&self, key: TagKey) -> Option<&str> {
        self.find_most_common_tag_value(key)
            .and_then(MostCommonItem::into_concensus)
    }

    /// Find album information for the given files.
    pub fn find_releases(&self) -> impl Stream<Item = crate::Result<MusicBrainzRelease>> + '_ {
        let artist = self.release_artist();
        let album = self.release_title();
        let artist_and_album = artist.and_then(|artist| album.map(|album| (artist, album)));

        if let Some((artist, album)) = artist_and_album {
            log::info!("Found artist and album: {} - {}", artist, album);
        };

        self.musicbrainz_release_id()
            .inspect(|mb_release_id| {
                log::info!("Found MusicBrainz Release Id: {:?}", mb_release_id);
            })
            .map_or_else(
                || future::ready(None).left_future(),
                |mb_id| {
                    async { find_release_by_mb_id(mb_id.to_string()).await.ok() }.right_future()
                },
            )
            .map(move |result| {
                if let Some(release) = result {
                    stream::once(future::ok(release)).left_stream()
                } else {
                    let mut query = MusicBrainzReleaseSearchQuery::query_builder();
                    let mut query = query.tracks(
                        &self
                            .track_count()
                            .map(|track_count| track_count.to_string())
                            .unwrap_or_default(),
                    );
                    if let Some(v) = artist {
                        query = query.and().artist(v);
                    };
                    if let Some(v) = album {
                        query = query.and().release(v);
                    };
                    if let Some(v) = self.catalog_number() {
                        query = query.and().catalog_number(v);
                    };
                    if let Some(v) = self.barcode() {
                        query = query.and().barcode(v);
                    }

                    let search = query.build();
                    async { MusicBrainzRelease::search(search).execute().await }
                        .map(|result| {
                            result.map_or_else(
                                |_| stream::empty().left_stream(),
                                |response| stream::iter(response.entities).right_stream(),
                            )
                        })
                        .flatten_stream()
                        .map(|release| release.id)
                        .then(find_release_by_mb_id)
                        .right_stream()
                }
            })
            .into_stream()
            .flatten()
    }
}

impl Release for TrackCollection {
    fn track_count(&self) -> Option<usize> {
        self.0.len().into()
    }

    fn release_title(&self) -> Option<&str> {
        self.find_consensual_tag_value(TagKey::Album)
    }

    fn release_artist(&self) -> Option<&str> {
        [TagKey::AlbumArtist, TagKey::Artist]
            .into_iter()
            .find_map(|key| self.find_most_common_tag_value(key))
            .and_then(|most_common_artist| {
                most_common_artist
                    .clone()
                    .into_concensus()
                    .map(|artist_name| {
                        if is_va_artist(artist_name) {
                            "Various Artists"
                        } else {
                            artist_name
                        }
                    })
                    .or_else(|| {
                        most_common_artist
                            .is_all_distinct()
                            .then_some("Various Artists")
                    })
            })
    }

    fn musicbrainz_release_id(&self) -> Option<&str> {
        self.find_consensual_tag_value(TagKey::MusicBrainzReleaseId)
    }

    fn catalog_number(&self) -> Option<&str> {
        self.find_consensual_tag_value(TagKey::CatalogNumber)
    }

    fn barcode(&self) -> Option<&str> {
        self.find_consensual_tag_value(TagKey::Barcode)
    }
}

/// Fetch a MusicBrainz release by its release ID.
pub async fn find_release_by_mb_id(id: String) -> crate::Result<MusicBrainzRelease> {
    MusicBrainzRelease::fetch()
        .id(&id)
        .with_artists()
        .with_recordings()
        .with_release_groups()
        .with_labels()
        .with_artist_credits()
        .with_aliases()
        .with_recording_level_relations()
        .with_work_relations()
        .with_work_level_relations()
        .with_artist_relations()
        .with_url_relations()
        .execute()
        .map(|result| result.map_err(crate::Error::from))
        .await
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
