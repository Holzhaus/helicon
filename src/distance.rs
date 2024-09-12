// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Distance calculations for various items.
use crate::release::Release;
use std::cmp;

/// A distance in the range (0.0, 1.0) between two items.
#[derive(Debug, Clone)]
pub struct Distance(f64);

impl Distance {
    /// Returns the distance between the items.
    pub fn as_f64(&self) -> f64 {
        debug_assert!(self.0.is_finite());
        debug_assert!(self.0 >= 0.0);
        debug_assert!(self.0 <= 1.0);
        if self.0.is_nan() {
            1.0
        } else {
            self.0.clamp(0.0, 1.0)
        }
    }
}

impl From<f64> for Distance {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<&[Distance]> for Distance {
    #[allow(clippy::cast_precision_loss)]
    fn from(value: &[Distance]) -> Self {
        let base_distance: f64 = value.iter().map(Distance::as_f64).sum();
        let total_distance = base_distance / (value.len() as f64);
        Distance::from(total_distance)
    }
}

impl PartialEq for Distance {
    fn eq(&self, other: &Self) -> bool {
        let lhs = self.as_f64();
        debug_assert!(!lhs.is_nan());
        let rhs = other.as_f64();
        debug_assert!(!rhs.is_nan());

        lhs == rhs
    }
}

impl Eq for Distance {}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Distance {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let lhs = self.as_f64();
        let rhs = other.as_f64();
        lhs.partial_cmp(&rhs).unwrap()
    }
}

mod string {
    //! Functions for distance calculation between strings.

    use super::Distance;
    use levenshtein::levenshtein;
    use std::cmp;
    use unidecode::unidecode;

    /// Common suffixes that are stripped and added as a suffix during [`Self::normalize`].
    const SUFFIXES: [&str; 3] = [", the", ", a", ", an"];

    /// Normalize a string slice value for comparison.
    fn normalize(value: &str) -> String {
        // Normalize all strings to ASCII lowercase.
        let mut value = unidecode(value);
        value.make_ascii_lowercase();

        // Move common suffixes (e.g., ", the") to the front of the string.
        for suffix in SUFFIXES {
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
    pub fn between(lhs: &str, rhs: &str) -> Distance {
        let lhs = normalize(lhs);
        let rhs = normalize(rhs);

        let levenshtein_distance = levenshtein(&lhs, &rhs);
        let max_possible_distance = cmp::max(lhs.len(), rhs.len());

        // FIXME: It's extremely unlikely, but this conversion to f64 is fallible. Hence, it should use
        // f64::try_from(usize) instead, but unfortunately that doesn't exist.
        Distance::from(levenshtein_distance as f64 / max_possible_distance as f64)
    }

    /// Calculate the distance between two string options.
    pub fn between_options(lhs: Option<&str>, rhs: Option<&str>) -> Distance {
        match (lhs, rhs) {
            (None, None) => Distance::from(0.0),
            (Some(_), None) | (None, Some(_)) => Distance::from(1.0),
            (Some(lhs), Some(rhs)) => between(lhs, rhs),
        }
    }
}

impl Distance {
    /// Calculate the distance between two string options.
    pub fn between_string_options(lhs: Option<&str>, rhs: Option<&str>) -> Self {
        string::between_options(lhs, rhs)
    }
}

mod release {
    //! Functions for distance calculation between [`Release`] objects.

    use super::Distance;
    use crate::release::Release;

    /// Calculate the distance between two releases.
    pub fn between<T1, T2>(lhs: &T1, rhs: &T2) -> Distance
    where
        T1: Release + ?Sized,
        T2: Release + ?Sized,
    {
        let release_title_distance =
            Distance::between_string_options(lhs.release_title(), rhs.release_title());
        let release_artist_distance =
            Distance::between_string_options(lhs.release_artist(), rhs.release_artist());

        let distances = [release_title_distance, release_artist_distance];
        Distance::from(distances.as_slice())
    }
}

impl Distance {
    /// Calculate the distance between two releases.
    pub fn between_releases<T1, T2>(lhs: &T1, rhs: &T2) -> Self
    where
        T1: Release + ?Sized,
        T2: Release + ?Sized,
    {
        release::between(lhs, rhs)
    }
}

/// An Item that is bundled together with its distance to a reference item.
pub struct DistanceItem<T> {
    /// The item.
    pub item: T,
    /// The distance of the item to a reference item (not part of this struct).
    pub distance: Distance,
}

impl<T> DistanceItem<T> {
    /// Create a new [`DistanceItem`].
    pub fn new(item: T, distance: Distance) -> Self {
        Self { item, distance }
    }

    /// The distance of the item to the reference item.
    pub fn distance(&self) -> &Distance {
        &self.distance
    }
}

impl<T> PartialEq for DistanceItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.distance().eq(other.distance())
    }
}

impl<T> Eq for DistanceItem<T> {}

impl<T> PartialOrd for DistanceItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for DistanceItem<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.distance().cmp(other.distance())
    }
}
