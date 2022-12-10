// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utilities for matching and lookup up albums and tracks.

use crate::tag::{TagKey, TaggedFile};
use std::collections::HashMap;

/// Finds the most common item in the iterator.
///
/// # Examples
///
/// The function returns `None` if the iterator is empty.
///
/// ```rust
/// assert_eq!(None, max_count(std::iter::empty::<String>()));
/// ```
///
/// In other cases, it returns the most common item, its count and the total number of values.
///
/// ```rust
/// let values = ["dog", "horse", "dog", "cat", "cat", "dog"];
/// assert_eq!(("dog", 3, 6), max_count(values.iter()));
/// ```
fn max_count<I, T>(items: I) -> Option<(T, usize, usize)>
where
    I: Iterator<Item = T>,
    T: Eq + std::hash::Hash,
{
    let mut counts = HashMap::new();
    items.for_each(|item| *counts.entry(item).or_insert(0) += 1);
    let total = counts.iter().map(|(_, count)| count).sum();
    counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(item, count)| (item, count, total))
}

/// Finds the most common value for a certain tag in an iterator of tagged files.
fn find_most_common_value<'a, I>(files: I, key: &TagKey) -> Option<(&'a str, usize, usize)>
where
    I: Iterator<Item = &'a TaggedFile>,
{
    max_count(
        files.filter_map(|tagged_file| tagged_file.tags().iter().find_map(|tag| tag.get(key))),
    )
}

/// Return `Some(value)` if the value is consensual, otherwise `None`
fn to_consensus<T>((value, count, total): (T, usize, usize)) -> Option<T> {
    (count == total).then_some(value)
}

/// Find artist and album from the given files.
fn find_artist_and_album(files: &[TaggedFile]) -> Option<(&str, &str)> {
    let artist = [TagKey::AlbumArtist, TagKey::Artist]
        .iter()
        .find_map(|key| find_most_common_value(files.iter(), key));

    let artist = artist
        .and_then(to_consensus)
        .map(|v| match v {
            "VA" | "Various" => "Various Artists",
            value => value,
        })
        .or_else(|| artist.and_then(|(_, count, _)| (count == 1).then_some("Various Artists")));

    let album = find_most_common_value(files.iter(), &TagKey::Album);

    artist.and_then(|artist| album.and_then(to_consensus).map(|album| (artist, album)))
}

/// Find the MusicBrainz release ID from the given files.
fn find_musicbrainz_release_id(files: &[TaggedFile]) -> Option<&str> {
    find_most_common_value(files.iter(), &TagKey::MusicBrainzAlbumId).and_then(to_consensus)
}

/// Find album information for the given files.
pub fn find_album_info(files: &[TaggedFile]) {
    if let Some((artist, album)) = find_artist_and_album(files) {
        log::info!("Found artist and album: {} - {}", artist, album);
    };

    if let Some(mb_album_id) = find_musicbrainz_release_id(files) {
        log::info!("Found MusicBrainz Release Id: {:?}", mb_album_id);
    };
}
