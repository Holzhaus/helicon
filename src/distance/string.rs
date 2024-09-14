// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

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
            let new_prefix = suffix.trim_start_matches(", ");
            value = format!("{new_prefix} {stripped}");
            break;
        }
    }

    // Replace ampersands with "and".
    value.replace('&', "and")
}

/// Calculate the case- and whitespace-insensitive distance between two strings, where 0.0 is
/// minimum and 1.0 is the maximum distance.
#[expect(clippy::cast_precision_loss)]
pub fn between(lhs: &str, rhs: &str) -> Distance {
    let lhs = normalize(lhs);
    let rhs = normalize(rhs);

    let levenshtein_distance = levenshtein(&lhs, &rhs);
    let max_possible_distance = cmp::max(lhs.len(), rhs.len());

    // FIXME: It's extremely unlikely, but this conversion to f64 is fallible. Hence, it should use
    // f64::try_from(usize) instead, but unfortunately that doesn't exist.
    Distance::from(levenshtein_distance as f64 / max_possible_distance as f64)
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use float_eq::assert_float_eq;

    #[test]
    fn test_string_distance_normalize_case() {
        let normalized = normalize("FoO bAr");
        assert_eq!("foo bar", normalized);
    }

    #[test]
    fn test_string_distance_normalize_suffix() {
        let normalized = normalize("Foo, The");
        assert_eq!("the foo", normalized);
    }

    #[test]
    fn test_string_distance_normalize_unicode() {
        let normalized = normalize("chopin's étude");
        assert_eq!("chopin's etude", normalized);
    }

    #[test]
    fn test_string_distance_exact() {
        let distance = between("foo", "foo");
        assert_float_eq!(distance.weighted_distance(), 0.0, abs <= 0.000_1);
    }

    #[test]
    fn test_string_distance_distinct() {
        let distance = between("foo", "bar");
        assert_float_eq!(distance.weighted_distance(), 1.0, abs <= 0.000_1);
    }

    #[test]
    fn test_string_distance_longer() {
        let distance = between("foo", "foobar");
        assert_float_eq!(distance.weighted_distance(), 0.5, abs <= 0.000_1);
    }

    #[test]
    fn test_string_distance_shorter() {
        let distance = between("foobar", "foo");
        assert_float_eq!(distance.weighted_distance(), 0.5, abs <= 0.000_1);
    }

    #[test]
    fn test_string_distance_suffix() {
        let distance = between("Foo & Bar, The", "The Foo and Bar");
        assert_float_eq!(distance.weighted_distance(), 0.0, abs <= 0.000_1);
    }
}
