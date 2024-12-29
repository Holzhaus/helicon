// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Distance calculations for various items.
use num::rational::Ratio;
use num::ToPrimitive;
use std::borrow::{Borrow, Cow};
use std::cmp;
use std::fmt;
use std::iter::Sum;

mod difference;
mod release;
mod string;
mod time;
mod track;

pub use difference::Difference;
pub use release::{ReleaseSimilarity, UnmatchedTracksSource};
pub use track::TrackSimilarity;

/// A distance in the range (0.0, 1.0) between two items.
#[expect(missing_copy_implementations)]
#[derive(Debug, Clone, PartialEq)]
pub struct Distance(f64);

impl Distance {
    /// Minimum distance (representing equality).
    pub const MIN: Distance = Distance(0.0);

    /// Maximum distance.
    pub const MAX: Distance = Distance(1.0);

    /// Return `true` if the distance is zero.
    pub const fn is_equality(&self) -> bool {
        self.0 == 0.0
    }

    /// Assigns a weight to the distance.
    pub fn into_weighted<'a>(self, weight: f64) -> WeightedDistance<'a> {
        debug_assert!(weight.is_finite());
        debug_assert!(weight >= 0.0);
        WeightedDistance {
            base_distance: Cow::Owned(self),
            weight,
        }
    }
    /// Assigns a weight to the distance.
    pub fn to_weighted(&self, weight: f64) -> WeightedDistance<'_> {
        debug_assert!(weight.is_finite());
        debug_assert!(weight >= 0.0);
        WeightedDistance {
            base_distance: Cow::Borrowed(self),
            weight,
        }
    }

    /// Returns the distance between the items as floating point number in the range 0 to 1.
    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

impl fmt::Display for Distance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_f64().fmt(f)
    }
}

/// A weighted version of the distance, that is used in calculations.
#[derive(Debug, Clone)]
pub struct WeightedDistance<'a> {
    /// The base distance.
    base_distance: Cow<'a, Distance>,
    /// The weight of the distance.
    weight: f64,
}

impl WeightedDistance<'_> {
    /// Changes the weight of the distance.
    pub fn with_weight(mut self, weight: f64) -> Self {
        debug_assert!(weight.is_finite());
        debug_assert!(weight >= 0.0);
        self.weight = weight;
        self
    }

    /// Returns the weight of the distance
    pub fn weight(&self) -> f64 {
        self.weight
    }

    /// Returns the distance between the items as floating point number in the range 0 to 1,
    /// multiplied with the weight.
    pub fn as_f64(&self) -> f64 {
        let value = self.base_distance.as_f64() * self.weight;
        debug_assert!(value.is_finite());
        value
    }
}

impl From<f64> for Distance {
    fn from(value: f64) -> Self {
        debug_assert!(value.is_finite());
        debug_assert!(value <= 1.0);
        debug_assert!(value >= 0.0);
        Self(value)
    }
}

impl From<bool> for Distance {
    fn from(value: bool) -> Self {
        Distance::from(if value { 0.0 } else { 1.0 })
    }
}

impl<T> From<Ratio<T>> for Distance
where
    Ratio<T>: ToPrimitive,
{
    fn from(value: Ratio<T>) -> Self {
        value.to_f64().map(Distance::from).unwrap()
    }
}

impl<'a> Sum<WeightedDistance<'a>> for Distance {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = WeightedDistance<'a>>,
    {
        let (total_distance, total_weight) = iter.fold(
            (0.0f64, 0.0f64),
            |(total_dist, total_weight), weighted_distance| {
                (
                    total_dist + weighted_distance.as_f64(),
                    total_weight + weighted_distance.weight,
                )
            },
        );

        Distance::from(total_distance / total_weight)
    }
}

impl Eq for Distance {}

impl Ord for Distance {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_f64().partial_cmp(&other.as_f64()).unwrap()
    }
}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Trait that allows to calculate a distance between two items.
///
/// This should only be implemented for simple items, where no additional configuration is needed.
pub trait DistanceBetween<S, T> {
    /// Calculate the distance between two items.
    fn between(lhs: S, rhs: T) -> Distance;
}

impl DistanceBetween<i64, i64> for Distance {
    fn between(lhs: i64, rhs: i64) -> Distance {
        Distance::from(Ratio::new(lhs, rhs))
    }
}

impl DistanceBetween<&str, &str> for Distance {
    fn between(lhs: &str, rhs: &str) -> Distance {
        string::between(lhs, rhs)
    }
}

impl DistanceBetween<Cow<'_, str>, Cow<'_, str>> for Distance {
    fn between(lhs: Cow<'_, str>, rhs: Cow<'_, str>) -> Distance {
        string::between(lhs.borrow(), rhs.borrow())
    }
}

impl DistanceBetween<chrono::TimeDelta, chrono::TimeDelta> for Distance {
    fn between(lhs: chrono::TimeDelta, rhs: chrono::TimeDelta) -> Distance {
        time::between(lhs, rhs)
    }
}

impl Distance {
    /// Return the distance between the two items, the maximum distance if one of them is `None` and
    /// the minimum distance if both are `None`.
    pub fn between_options_or_minmax<S, T>(lhs: Option<S>, rhs: Option<T>) -> Distance
    where
        Self: DistanceBetween<S, T>,
    {
        match (lhs, rhs) {
            (None, None) => Distance::from(0.0),
            (Some(_), None) | (None, Some(_)) => Distance::from(1.0),
            (Some(lhs), Some(rhs)) => Distance::between(lhs, rhs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_eq::assert_float_eq;

    #[test]
    fn test_distance_from_slice() {
        let dist0 = Distance::from(1.0);
        let dist1 = Distance::from(0.2);
        let dist2 = Distance::from(0.5);
        let dist3 = Distance::from(0.45);
        let dist4 = Distance::from(0.35);

        let total: Distance = [dist0, dist1, dist2, dist3, dist4]
            .iter()
            .map(|dist| dist.to_weighted(1.0))
            .sum();
        assert_float_eq!(total.as_f64(), 0.5, abs <= 0.000_1);
    }

    #[test]
    fn test_distance_from_slice_weighted() {
        let dist0 = Distance::from(1.0);
        let dist1 = Distance::from(0.2);
        let dist2 = Distance::from(0.5);
        let dist3 = Distance::from(0.45);
        let dist4 = Distance::from(0.55);

        let total: Distance = [
            dist0.to_weighted(2.5),
            dist1.to_weighted(5.0),
            dist2.to_weighted(0.5),
            dist3.to_weighted(3.0),
            dist4.to_weighted(5.0),
        ]
        .into_iter()
        .sum();
        assert_float_eq!(total.as_f64(), 0.490_625, abs <= 0.000_1);
    }

    #[test]
    fn test_distance_ord_impl() {
        let dist0 = Distance::from(0.000);
        let dist1 = Distance::from(0.001);
        let dist2 = Distance::from(0.002);

        assert_eq!(dist0.cmp(&dist0), cmp::Ordering::Equal);
        assert_eq!(dist1.cmp(&dist1), cmp::Ordering::Equal);
        assert_eq!(dist2.cmp(&dist2), cmp::Ordering::Equal);

        assert_eq!(dist0.cmp(&dist1), cmp::Ordering::Less);
        assert_eq!(dist0.cmp(&dist2), cmp::Ordering::Less);
        assert_eq!(dist1.cmp(&dist2), cmp::Ordering::Less);

        assert_eq!(dist1.cmp(&dist0), cmp::Ordering::Greater);
        assert_eq!(dist2.cmp(&dist0), cmp::Ordering::Greater);
        assert_eq!(dist2.cmp(&dist1), cmp::Ordering::Greater);
    }

    #[test]
    fn test_distance_partialord_impl() {
        let dist0 = Distance::from(0.000);
        let dist1 = Distance::from(0.001);
        let dist2 = Distance::from(0.002);

        assert_eq!(dist0.partial_cmp(&dist0).unwrap(), cmp::Ordering::Equal);
        assert_eq!(dist1.partial_cmp(&dist1).unwrap(), cmp::Ordering::Equal);
        assert_eq!(dist2.partial_cmp(&dist2).unwrap(), cmp::Ordering::Equal);

        assert_eq!(dist0.partial_cmp(&dist1).unwrap(), cmp::Ordering::Less);
        assert_eq!(dist0.partial_cmp(&dist2).unwrap(), cmp::Ordering::Less);
        assert_eq!(dist1.partial_cmp(&dist2).unwrap(), cmp::Ordering::Less);

        assert_eq!(dist1.partial_cmp(&dist0).unwrap(), cmp::Ordering::Greater);
        assert_eq!(dist2.partial_cmp(&dist0).unwrap(), cmp::Ordering::Greater);
        assert_eq!(dist2.partial_cmp(&dist1).unwrap(), cmp::Ordering::Greater);
    }
}
