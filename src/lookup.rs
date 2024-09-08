// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utilities for matching and lookup up albums and tracks.

use crate::tag::{TagKey, TaggedFile};
use levenshtein::levenshtein;
use musicbrainz_rs_nova::{
    entity::release::{Release, ReleaseSearchQuery},
    Fetch, Search,
};
use std::cmp;
use std::collections::HashMap;
use unidecode::unidecode;

/// Finds the most common item in the iterator.
///
/// The function returns `None` if the iterator is empty. In other cases, it returns the most
/// common item, its count and the total number of values.
fn max_count<I, T>(items: I) -> Option<(T, usize, usize)>
where
    I: Iterator<Item = T>,
    T: Eq + std::hash::Hash,
{
    let mut counts = HashMap::new();
    items.for_each(|item| *counts.entry(item).or_insert(0) += 1);
    let total = counts.values().sum();
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

/// Finds the consensual value for a certain tag in an iterator of tagged files.
///
/// Returns `None` if there is no consensual value.
fn find_consensual_value<'a, I>(files: I, key: &TagKey) -> Option<&'a str>
where
    I: Iterator<Item = &'a TaggedFile>,
{
    find_most_common_value(files, key).and_then(to_consensus)
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

/// Find artist from the given files.
fn find_artist(files: &[TaggedFile]) -> Option<&str> {
    let artist = [TagKey::AlbumArtist, TagKey::Artist]
        .iter()
        .find_map(|key| find_most_common_value(files.iter(), key));

    artist
        .and_then(to_consensus)
        .map(|v| match v {
            "VA" | "Various" => "Various Artists",
            value => value,
        })
        .or_else(|| artist.and_then(|(_, count, _)| (count == 1).then_some("Various Artists")))
}

/// Find the MusicBrainz release ID from the given files.
fn find_musicbrainz_release_id(files: &[TaggedFile]) -> Option<&str> {
    find_most_common_value(files.iter(), &TagKey::MusicBrainzReleaseId).and_then(to_consensus)
}

/// Find album information for the given files.
pub async fn find_album_info(files: &[TaggedFile]) -> crate::Result<Vec<Release>> {
    let artist = find_artist(files);
    let album = find_consensual_value(files.iter(), &TagKey::Album);
    let artist_and_album = artist.and_then(|artist| album.map(|album| (artist, album)));

    if let Some((artist, album)) = artist_and_album {
        log::info!("Found artist and album: {} - {}", artist, album);
    };

    if let Some(mb_album_id) = find_musicbrainz_release_id(files) {
        log::info!("Found MusicBrainz Release Id: {:?}", mb_album_id);
        match Release::fetch().id(mb_album_id).execute().await {
            Ok(release) => return Ok(vec![release]),
            Err(err) => log::warn!("Failed to fetch musicbrainz release: {:?}", err),
        };
    };

    let tracks = format!("{}", files.len());
    let mut query = ReleaseSearchQuery::query_builder();
    let mut query = query.tracks(&tracks);
    if let Some(v) = artist {
        query = query.and().artist(v);
    };
    if let Some(v) = album {
        query = query.and().release(v);
    };
    if let Some(v) = find_consensual_value(files.iter(), &TagKey::CatalogNumber) {
        query = query.and().catalog_number(v);
    };
    if let Some(v) = find_consensual_value(files.iter(), &TagKey::Barcode) {
        query = query.and().barcode(v);
    };

    Ok(Release::search(query.build()).execute().await?.entities)
}
#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_max_count_empty_iterator() {
        assert_eq!(None, max_count(std::iter::empty::<String>()));
    }

    #[test]
    fn test_max_count_with_strings() {
        let values = ["dog", "horse", "dog", "cat", "cat", "dog"];
        assert_eq!(Some((&"dog", 3, 6)), max_count(values.iter()));
    }

    #[test]
    fn test_max_count_with_integers() {
        let values = [1, 2, 3, 4, 5, 1, 2, 3, 2, 5, 9, 8, 2];
        assert_eq!(Some((&2, 4, 13)), max_count(values.iter()));
    }
}
