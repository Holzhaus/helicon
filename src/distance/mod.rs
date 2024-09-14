// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Distance calculations for various items.
use crate::release::ReleaseLike;
use crate::track::TrackLike;
use std::borrow::{Borrow, Cow};
use std::cmp;

mod release;
mod string;
mod track;

/// A distance in the range (0.0, 1.0) between two items.
#[derive(Debug, Clone, PartialEq)]
pub struct Distance {
    /// The unweighted base distance.
    base_distance: f64,
    /// The weight.
    weight: f64,
}

impl Distance {
    /// Assigns a weight to the distance.
    pub fn with_weight(mut self, weight: f64) -> Self {
        debug_assert!(weight.is_finite());
        debug_assert!(weight >= 0.0);
        self.weight = weight;
        self
    }

    /// Returns the distance between the items.
    pub fn weighted_distance(&self) -> f64 {
        let weighted_distance = self.base_distance * self.weight;
        debug_assert!(weighted_distance.is_finite());
        weighted_distance
    }

    /// Returns the weight of the distance
    pub fn weight(&self) -> f64 {
        self.weight
    }

    /// Calculate distance between two [`ReleaseLike`] items.
    pub fn between_releases(lhs: &impl ReleaseLike, rhs: &impl ReleaseLike) -> Self {
        release::between(lhs, rhs)
    }

    /// Calculate distance between two [`TrackLike`] items.
    pub fn between_tracks(lhs: &impl TrackLike, rhs: &impl TrackLike) -> Self {
        track::between(lhs, rhs)
    }
}

impl From<f64> for Distance {
    fn from(value: f64) -> Self {
        debug_assert!(value.is_finite());
        debug_assert!(value <= 1.0);
        debug_assert!(value >= 0.0);
        Self {
            base_distance: value,
            weight: 1.0,
        }
    }
}

impl From<bool> for Distance {
    fn from(value: bool) -> Self {
        Distance::from(if value { 0.0 } else { 1.0 })
    }
}

impl From<&[Distance]> for Distance {
    fn from(value: &[Distance]) -> Self {
        let total_weighted_distance: f64 = value.iter().map(Distance::weighted_distance).sum();
        let total_weight: f64 = value.iter().map(Distance::weight).sum();
        let total_distance = total_weighted_distance / total_weight;
        Distance::from(total_distance)
    }
}

impl Eq for Distance {}

impl Ord for Distance {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.weighted_distance()
            .partial_cmp(&other.weighted_distance())
            .unwrap()
    }
}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Trait that allows to calculate a distance between two items.
pub trait DistanceBetween<S, T> {
    /// Calculate the distance between two items.
    fn between(lhs: S, rhs: T) -> Distance;
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

impl<S, T> DistanceBetween<Option<S>, Option<T>> for Distance
where
    Self: DistanceBetween<S, T>,
{
    fn between(lhs: Option<S>, rhs: Option<T>) -> Distance {
        match (lhs, rhs) {
            (None, None) => Distance::from(0.0),
            (Some(_), None) | (None, Some(_)) => Distance::from(1.0),
            (Some(lhs), Some(rhs)) => Distance::between(lhs, rhs),
        }
    }
}

/// An Item that is bundled together with its distance to a reference item.
#[derive(Debug, Clone)]
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

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use float_eq::assert_float_eq;

    #[test]
    fn test_distance_from_slice() {
        let dist0 = Distance::from(1.0);
        let dist1 = Distance::from(0.2);
        let dist2 = Distance::from(0.5);
        let dist3 = Distance::from(0.45);
        let dist4 = Distance::from(0.35);

        let total = Distance::from([dist0, dist1, dist2, dist3, dist4].as_slice());
        assert_float_eq!(total.weighted_distance(), 0.5, abs <= 0.000_1);
    }

    #[test]
    fn test_distance_from_slice_weighted() {
        let dist0 = Distance::from(1.0).with_weight(2.5);
        let dist1 = Distance::from(0.2).with_weight(5.0);
        let dist2 = Distance::from(0.5).with_weight(0.5);
        let dist3 = Distance::from(0.45).with_weight(3.0);
        let dist4 = Distance::from(0.55).with_weight(5.0);

        let total = Distance::from([dist0, dist1, dist2, dist3, dist4].as_slice());
        assert_float_eq!(total.weighted_distance(), 0.490_625, abs <= 0.000_1);
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

    #[test]
    fn test_distanceitem_ord_impl() {
        let item0 = DistanceItem::new((), Distance::from(0.000));
        let item1 = DistanceItem::new((), Distance::from(0.001));
        let item2 = DistanceItem::new((), Distance::from(0.002));

        assert_eq!(item0.cmp(&item0), cmp::Ordering::Equal);
        assert_eq!(item1.cmp(&item1), cmp::Ordering::Equal);
        assert_eq!(item2.cmp(&item2), cmp::Ordering::Equal);

        assert_eq!(item0.cmp(&item1), cmp::Ordering::Less);
        assert_eq!(item0.cmp(&item2), cmp::Ordering::Less);
        assert_eq!(item1.cmp(&item2), cmp::Ordering::Less);

        assert_eq!(item1.cmp(&item0), cmp::Ordering::Greater);
        assert_eq!(item2.cmp(&item0), cmp::Ordering::Greater);
        assert_eq!(item2.cmp(&item1), cmp::Ordering::Greater);
    }

    #[test]
    fn test_distanceitem_partialord_impl() {
        let item0 = DistanceItem::new((), Distance::from(0.000));
        let item1 = DistanceItem::new((), Distance::from(0.001));
        let item2 = DistanceItem::new((), Distance::from(0.002));

        assert_eq!(item0.partial_cmp(&item0).unwrap(), cmp::Ordering::Equal);
        assert_eq!(item1.partial_cmp(&item1).unwrap(), cmp::Ordering::Equal);
        assert_eq!(item2.partial_cmp(&item2).unwrap(), cmp::Ordering::Equal);

        assert_eq!(item0.partial_cmp(&item1).unwrap(), cmp::Ordering::Less);
        assert_eq!(item0.partial_cmp(&item2).unwrap(), cmp::Ordering::Less);
        assert_eq!(item1.partial_cmp(&item2).unwrap(), cmp::Ordering::Less);

        assert_eq!(item1.partial_cmp(&item0).unwrap(), cmp::Ordering::Greater);
        assert_eq!(item2.partial_cmp(&item0).unwrap(), cmp::Ordering::Greater);
        assert_eq!(item2.partial_cmp(&item1).unwrap(), cmp::Ordering::Greater);
    }
}
