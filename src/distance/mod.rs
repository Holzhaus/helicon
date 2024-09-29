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
use std::iter::Sum;

mod release;
mod string;
mod time;
mod track;

pub use release::{ReleaseSimilarity, UnmatchedTracksSource};
pub use track::TrackSimilarity;

/// A distance in the range (0.0, 1.0) between two items.
#[expect(missing_copy_implementations)]
#[derive(Debug, Clone, PartialEq)]
pub struct Distance {
    /// The unweighted base distance.
    base_distance: f64,
    /// The weight.
    weight: f64,
}

impl Distance {
    /// Return `true` if the distance is zero.
    pub fn is_equality(&self) -> bool {
        self.base_distance == 0.0
    }

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

    /// Calculate distance between two tuple items.
    pub fn between_tuple_items<T, S>((lhs, rhs): (T, S)) -> Self
    where
        Self: DistanceBetween<T, S>,
    {
        Distance::between(lhs, rhs)
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

impl<T> From<Ratio<T>> for Distance
where
    Ratio<T>: ToPrimitive,
{
    fn from(value: Ratio<T>) -> Self {
        value.to_f64().map(Distance::from).unwrap()
    }
}

impl<'a> Sum<&'a Distance> for Distance {
    // Required method
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Distance>,
    {
        let (total_weighted_dist, total_weight) =
            iter.fold((0.0f64, 0.0f64), |(weighted_dist, weight), distance| {
                (
                    weighted_dist + distance.weighted_distance(),
                    weight + distance.weight,
                )
            });

        Distance::from(total_weighted_dist / total_weight)
    }
}

impl Sum<Distance> for Distance {
    // Required method
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Distance>,
    {
        let (total_weighted_dist, total_weight) =
            iter.fold((0.0f64, 0.0f64), |(weighted_dist, weight), distance| {
                (
                    weighted_dist + distance.weighted_distance(),
                    weight + distance.weight,
                )
            });

        Distance::from(total_weighted_dist / total_weight)
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
    use super::*;
    use crate::track::TrackLike;
    use float_eq::assert_float_eq;

    pub struct TestTrack(pub &'static str);
    impl TrackLike for TestTrack {
        fn acoustid(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn arranger(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn track_artist(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn track_artist_sort_order(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn bpm(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn comment(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn composer(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn composer_sort_order(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn conductor(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn copyright(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn director(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn dj_mixer(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn encoded_by(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn encoder_settings(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn engineer(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn genre(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn initial_key(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn isrc(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn language(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn license(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn lyricist(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn lyrics(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn mixer(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn mood(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn movement(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn movement_count(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn movement_number(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicbrainz_artist_id(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicbrainz_original_artist_id(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicbrainz_original_release_id(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicbrainz_track_id(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicbrainz_trm_id(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicbrainz_work_id(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicip_fingerprint(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn musicip_puid(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn original_album(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn original_artist(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn original_filename(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn original_release_date(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn original_release_year(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn performer(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn producer(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn rating(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn remixer(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn replay_gain_album_gain(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn replay_gain_album_peak(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn replay_gain_album_range(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn replay_gain_reference_loudness(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn replay_gain_track_range(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn track_number(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn track_title(&self) -> Option<Cow<'_, str>> {
            Cow::from(self.0).into()
        }

        fn track_title_sort_order(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn artist_website(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn work_title(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn writer(&self) -> Option<Cow<'_, str>> {
            None
        }

        fn track_length(&self) -> Option<chrono::TimeDelta> {
            None
        }
    }

    #[test]
    fn test_distance_from_slice() {
        let dist0 = Distance::from(1.0);
        let dist1 = Distance::from(0.2);
        let dist2 = Distance::from(0.5);
        let dist3 = Distance::from(0.45);
        let dist4 = Distance::from(0.35);

        let total: Distance = [dist0, dist1, dist2, dist3, dist4].into_iter().sum();
        assert_float_eq!(total.weighted_distance(), 0.5, abs <= 0.000_1);
    }

    #[test]
    fn test_distance_from_slice_weighted() {
        let dist0 = Distance::from(1.0).with_weight(2.5);
        let dist1 = Distance::from(0.2).with_weight(5.0);
        let dist2 = Distance::from(0.5).with_weight(0.5);
        let dist3 = Distance::from(0.45).with_weight(3.0);
        let dist4 = Distance::from(0.55).with_weight(5.0);

        let total: Distance = [dist0, dist1, dist2, dist3, dist4].into_iter().sum();
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
