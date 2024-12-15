// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! The [`Difference`] type represents the difference between a specific information between two
//! items (e.g., two tracks).

use super::{Distance, DistanceBetween};

/// Difference between two metadata items.
#[derive(Debug, Clone, PartialEq)]
pub enum Difference {
    /// This item is missing on both sides.
    BothMissing,
    /// This item is missing on the left hand side, but present on the right hand side.
    Added,
    /// This item is present on the left hand side, but missing on the right hand side.
    Removed,
    /// This item is present on both sides. If and how much the value differs is determined by the
    /// distance.
    BothPresent(Distance),
}

impl Difference {
    /// Return the distance between the two items, the maximum distance if one of them is `None` and
    /// the minimum distance if both are `None`.
    pub fn between_options_fn<L, R, F>(lhs: Option<L>, rhs: Option<R>, f: F) -> Self
    where
        F: FnOnce(L, R) -> Distance,
    {
        match (lhs, rhs) {
            (None, None) => Self::BothMissing,
            (Some(_), None) => Self::Removed,
            (None, Some(_)) => Self::Added,
            (Some(lhs), Some(rhs)) => Self::BothPresent(f(lhs, rhs)),
        }
    }

    /// Return the distance between the two items, the maximum distance if one of them is `None` and
    /// the minimum distance if both are `None`.
    pub fn between_options<L, R>(lhs: Option<L>, rhs: Option<R>) -> Self
    where
        Distance: DistanceBetween<L, R>,
    {
        Self::between_options_fn(lhs, rhs, Distance::between)
    }

    /// Get the distance. Added or Removed values are mapped to the maximum distance.
    pub const fn to_distance(&self) -> &Distance {
        match &self {
            Self::BothMissing => &Distance::MIN,
            Self::Added | Self::Removed => &Distance::MAX,
            Self::BothPresent(dist) => dist,
        }
    }

    /// Get the distance as [`Option`]. This method will return the distance in case that that both
    /// values are present, otherwise it will return `None`.
    pub const fn to_distance_if_both_present(&self) -> Option<&Distance> {
        match &self {
            Self::BothMissing | Self::Added | Self::Removed => None,
            Self::BothPresent(dist) => Some(dist),
        }
    }

    /// Returns true if the value is present on the left hand side.
    pub fn is_present_left(&self) -> bool {
        matches!(self, Self::Removed | Self::BothPresent(_))
    }

    /// Convenience method that returns  `true` if either both values are missing or both are
    /// present and equal.
    pub const fn is_equal(&self) -> bool {
        self.to_distance().is_equality()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_between_options_both_missing() {
        let diff = Difference::between_options::<&str, &str>(None, None);
        assert_eq!(diff, Difference::BothMissing);
    }

    #[test]
    fn test_between_options_added() {
        let diff = Difference::between_options(None, Some("foo"));
        assert_eq!(diff, Difference::Added);
    }

    #[test]
    fn test_between_options_removed() {
        let diff = Difference::between_options(Some("foo"), None);
        assert_eq!(diff, Difference::Removed);
    }

    #[test]
    fn test_between_options_both_present_min_dist() {
        let diff = Difference::between_options(Some("foo"), Some("foo"));
        assert_eq!(diff, Difference::BothPresent(Distance::MIN));
    }

    #[test]
    fn test_between_options_both_present_max_dist() {
        let diff = Difference::between_options(Some("foo"), Some("bar"));
        assert_eq!(diff, Difference::BothPresent(Distance::MAX));
    }

    #[test]
    fn test_between_options_both_present_medium_dist() {
        let diff = Difference::between_options(Some("foo"), Some("foobar"));
        assert_eq!(diff, Difference::BothPresent(Distance::from(0.5)));
    }

    #[test]
    fn test_between_options_fn() {
        let diff = Difference::between_options_fn(Some(1), Some(2), |a: i32, b: i32| {
            Distance::from(a.is_positive() == b.is_positive())
        });
        assert_eq!(diff, Difference::BothPresent(Distance::MIN));
    }

    #[test]
    fn test_to_distance_both_missing() {
        let diff = Difference::BothMissing;
        debug_assert_eq!(diff.to_distance(), &Distance::MIN);
        debug_assert_eq!(diff.to_distance_if_both_present(), None);
    }

    #[test]
    fn test_to_distance_added() {
        let diff = Difference::Added;
        debug_assert_eq!(diff.to_distance(), &Distance::MAX);
        debug_assert_eq!(diff.to_distance_if_both_present(), None);
    }

    #[test]
    fn test_to_distance_removed() {
        let diff = Difference::Removed;
        debug_assert_eq!(diff.to_distance(), &Distance::MAX);
        debug_assert_eq!(diff.to_distance_if_both_present(), None);
    }

    #[test]
    fn test_to_distance_both_present() {
        let dist = Distance::from(0.5);
        let diff = Difference::BothPresent(dist.clone());
        debug_assert_eq!(diff.to_distance(), &dist);
        debug_assert_eq!(diff.to_distance_if_both_present(), Some(diff.to_distance()));
    }
}
