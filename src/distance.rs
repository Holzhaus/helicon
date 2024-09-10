// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Distance calculations for various items.
use crate::release::Release;
use levenshtein::levenshtein;
use std::cmp;
use unidecode::unidecode;

/// A distance between two items.
pub trait Distance {
    /// Returns the distance between the items.
    fn distance(&self) -> f64;
}

/// A distance between two strings.
struct StringDistance(f64);

impl StringDistance {
    /// Common suffixes that are stripped and added as a suffix during [`Self::normalize`].
    const SUFFIXES: [&str; 3] = [", the", ", a", ", an"];

    /// Normalize a string slice value for comparison.
    fn normalize(value: &str) -> String {
        // Normalize all strings to ASCII lowercase.
        let mut value = unidecode(value);
        value.make_ascii_lowercase();

        // Move common suffixes (e.g., ", the") to the front of the string.
        for suffix in Self::SUFFIXES {
            if let Some(stripped) = value.strip_suffix(suffix) {
                let new_prefix = stripped.trim_start_matches(", ");
                value = format!("{new_prefix} {value}");
                break;
            }
        }

        // Replace ampersands with "and".
        value.replace('&', "and")
    }

    /// Calculate the case- and whitespace-insensitive distance between two strings, where 0.0 is
    /// minimum and 1.0 is the maximum distance.
    #[allow(clippy::cast_precision_loss)]
    fn calculate_distance(lhs: &str, rhs: &str) -> f64 {
        let levenshtein_distance = levenshtein(lhs, rhs);
        let max_possible_distance = cmp::max(lhs.len(), rhs.len());

        // FIXME: It's extremely unlikely, but this conversion to f64 is fallible. Hence, it should use
        // f64::try_from(usize) instead, but unfortunately that doesn't exist.
        levenshtein_distance as f64 / max_possible_distance as f64
    }

    /// Calculate the distance between two strings.
    pub fn between(lhs: Option<&str>, rhs: Option<&str>) -> Self {
        match (lhs, rhs) {
            (None, None) => Self(0.0),
            (Some(_), None) | (None, Some(_)) => Self(1.0),
            (Some(lhs), Some(rhs)) => {
                let lhs = Self::normalize(lhs);
                let rhs = Self::normalize(rhs);

                let distance = Self::calculate_distance(&lhs, &rhs);
                Self(distance)
            }
        }
    }
}

impl Distance for StringDistance {
    fn distance(&self) -> f64 {
        self.0
    }
}

/// Distance between two releases.
pub struct ReleaseDistance {
    /// Distance of the release titles.
    release_title_distance: StringDistance,
    /// Distance of the release artists.
    release_artist_distance: StringDistance,
}

impl ReleaseDistance {
    /// Calculate the distance between two releases.
    pub fn between<T1, T2>(lhs: &T1, rhs: &T2) -> Self
    where
        T1: Release + ?Sized,
        T2: Release + ?Sized,
    {
        let release_title_distance =
            StringDistance::between(lhs.release_title(), rhs.release_title());
        let release_artist_distance =
            StringDistance::between(lhs.release_artist(), rhs.release_artist());

        Self {
            release_title_distance,
            release_artist_distance,
        }
    }
}

impl Distance for ReleaseDistance {
    #[allow(clippy::cast_precision_loss)]
    fn distance(&self) -> f64 {
        let distances = [&self.release_title_distance, &self.release_artist_distance];
        let base_distance: f64 = distances.iter().map(|item| item.distance()).sum();
        base_distance / (distances.len() as f64)
    }
}
