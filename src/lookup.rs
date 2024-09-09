// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utilities for matching and lookup up albums and tracks.

use crate::tag::{TagKey, TaggedFile};
use futures::{
    future::{self, FutureExt},
    stream::{self, StreamExt},
    Stream,
};
use levenshtein::levenshtein;
use musicbrainz_rs_nova::{
    entity::release::{Release, ReleaseSearchQuery},
    Fetch, Search,
};
use std::cmp;
use std::collections::HashMap;
use unidecode::unidecode;

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

/// Finds the most common value for a certain tag in an iterator of tagged files.
fn find_most_common_tag_value<'a, I>(files: I, key: &TagKey) -> Option<MostCommonItem<&'a str>>
where
    I: Iterator<Item = &'a TaggedFile>,
{
    MostCommonItem::find(
        files.filter_map(|tagged_file| tagged_file.tags().iter().find_map(|tag| tag.get(key))),
    )
}

/// Finds the consensual value for a certain tag in an iterator of tagged files.
///
/// Returns `None` if there is no consensual value.
fn find_consensual_tag_value<'a, I>(files: I, key: &TagKey) -> Option<&'a str>
where
    I: Iterator<Item = &'a TaggedFile>,
{
    find_most_common_tag_value(files, key).and_then(MostCommonItem::into_concensus)
}

/// Calculate the case- and whitespace-insensitive distance between two strings, where 0.0 is
/// minimum and 1.0 is the maximum distance.
#[allow(clippy::cast_precision_loss)]
#[allow(dead_code)]
fn string_distance(lhs: &str, rhs: &str) -> f64 {
    let mut lhs = unidecode(lhs);
    lhs.retain(|c| c.is_ascii_alphanumeric());
    lhs.make_ascii_lowercase();

    let mut rhs = unidecode(rhs);
    rhs.retain(|c| c.is_ascii_alphanumeric());
    rhs.make_ascii_lowercase();

    if lhs.is_empty() && rhs.is_empty() {
        return 0.0;
    }

    let levenshtein_distance = levenshtein(&lhs, &rhs);
    let max_possible_distance = cmp::max(lhs.len(), rhs.len());

    // FIXME: It's extremely unlikely, but this conversion to f64 is fallible. Hence, it should use
    // f64::try_from(usize) instead, but unfortunately that doesn't exist.
    levenshtein_distance as f64 / max_possible_distance as f64
}

/// Returns `true` if the artist is likely "Various Artists".
fn is_va_artist(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "" | "various artists" | "various" | "va" | "unknown"
    )
}

/// Find artist from the given files.
fn find_artist(files: &[TaggedFile]) -> Option<&str> {
    [TagKey::AlbumArtist, TagKey::Artist]
        .iter()
        .find_map(|key| find_most_common_tag_value(files.iter(), key))
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

/// Find the MusicBrainz release ID from the given files.
fn find_musicbrainz_release_id(files: &[TaggedFile]) -> Option<&str> {
    find_consensual_tag_value(files.iter(), &TagKey::MusicBrainzReleaseId)
}

/// Find album information for the given files.
pub fn find_releases(files: &[TaggedFile]) -> impl Stream<Item = crate::Result<Release>> + '_ {
    let artist = find_artist(files);
    let album = find_consensual_tag_value(files.iter(), &TagKey::Album);
    let artist_and_album = artist.and_then(|artist| album.map(|album| (artist, album)));

    if let Some((artist, album)) = artist_and_album {
        log::info!("Found artist and album: {} - {}", artist, album);
    };

    find_musicbrainz_release_id(files)
        .inspect(|mb_release_id| {
            log::info!("Found MusicBrainz Release Id: {:?}", mb_release_id);
        })
        .map_or_else(
            || future::ready(None).left_future(),
            |mb_id| async { find_release_by_mb_id(mb_id.to_string()).await.ok() }.right_future(),
        )
        .map(move |result| {
            if let Some(release) = result {
                stream::once(future::ok(release)).left_stream()
            } else {
                let tracks = format!("{}", files.len());
                let mut query = ReleaseSearchQuery::query_builder();
                let mut query = query.tracks(&tracks);
                if let Some(v) = artist {
                    query = query.and().artist(v);
                };
                if let Some(v) = album {
                    query = query.and().release(v);
                };
                if let Some(v) = find_consensual_tag_value(files.iter(), &TagKey::CatalogNumber) {
                    query = query.and().catalog_number(v);
                };
                if let Some(v) = find_consensual_tag_value(files.iter(), &TagKey::Barcode) {
                    query = query.and().barcode(v);
                }

                let search = query.build();
                async { Release::search(search).execute().await }
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

/// Fetch a MusicBrainz release by its release ID.
pub async fn find_release_by_mb_id(id: String) -> crate::Result<Release> {
    Release::fetch()
        .id(&id)
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
