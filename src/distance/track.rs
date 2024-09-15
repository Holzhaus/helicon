// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions for distance calculation between [`ReleaseLike`] objects.

use super::{Distance, DistanceBetween};
use crate::track::TrackLike;
use std::borrow::Borrow;

/// Calculate the distance between two releases.
pub fn between<T1, T2>(lhs: &T1, rhs: &T2) -> Distance
where
    T1: TrackLike + ?Sized,
    T2: TrackLike + ?Sized,
{
    let track_title_distance =
        Distance::between(lhs.track_title(), rhs.track_title()).with_weight(3.0);
    let track_artist_distance = rhs
        .track_artist()
        .map(|rhs_artist| Distance::between(lhs.track_artist(), Some(rhs_artist)).with_weight(3.0));
    let track_number_distance = lhs
        .track_number()
        .and_then(|lhs_len| rhs.track_number().map(|rhs_len| (lhs_len, rhs_len)))
        .map(|(lhs_len, rhs_len)| Distance::between(lhs_len, rhs_len));
    let track_length_distance = lhs
        .track_length()
        .and_then(|lhs_len| rhs.track_length().map(|rhs_len| (lhs_len, rhs_len)))
        .map(|(lhs_len, rhs_len)| Distance::between(lhs_len, rhs_len));
    let musicbrainz_recording_id_distance = lhs
        .musicbrainz_recording_id()
        .and_then(|lhs_id| {
            rhs.musicbrainz_recording_id()
                .map(|rhs_id| (lhs_id, rhs_id))
        })
        .map(|(lhs, rhs)| {
            let lhs_id: &str = lhs.borrow();
            let lhs_id: &str = lhs_id.trim();

            let rhs_id: &str = rhs.borrow();
            let rhs_id: &str = rhs_id.trim();

            Distance::from(lhs_id == rhs_id && !lhs_id.is_empty()).with_weight(5.0)
        });

    let distances: Vec<_> = [
        Some(track_title_distance),
        track_artist_distance,
        track_number_distance,
        track_length_distance,
        musicbrainz_recording_id_distance,
    ]
    .into_iter()
    .flatten()
    .collect();
    Distance::from(distances.as_slice())
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::super::tests::TestTrack;
    use super::*;
    use float_eq::assert_float_eq;

    #[test]
    fn test_track_distance_title_exact() {
        let track = TestTrack("foo");
        let distance = between(&track, &track);
        assert_float_eq!(distance.weighted_distance(), 0.0, abs <= 0.000_1);
    }

    #[test]
    fn test_track_distance_title_distinct() {
        let track1 = TestTrack("foo");
        let track2 = TestTrack("bar");
        let distance = between(&track1, &track2);
        assert_float_eq!(distance.weighted_distance(), 1.0, abs <= 0.000_1);
    }

    #[test]
    fn test_track_distance_title_similar() {
        let track1 = TestTrack("foo");
        let track2 = TestTrack("barfoo");
        let distance = between(&track1, &track2);
        assert_float_eq!(distance.weighted_distance(), 0.5, abs <= 0.000_1);
    }
}
