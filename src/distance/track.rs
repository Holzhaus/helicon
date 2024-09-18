// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions for distance calculation between [`ReleaseLike`] objects.

use super::{string, Distance};
use crate::track::TrackLike;

/// Calculate the distance between two releases.
pub fn between<T1, T2>(lhs: &T1, rhs: &T2) -> Distance
where
    T1: TrackLike + ?Sized,
    T2: TrackLike + ?Sized,
{
    let track_title_distance =
        Distance::between_options_or_maximum(lhs.track_title(), rhs.track_title()).with_weight(3.0);
    let track_artist_distance =
        Distance::between_options_if_both_some(lhs.track_artist(), rhs.track_artist())
            .map(|distance| distance.with_weight(3.0));
    let track_number_distance =
        Distance::between_options_if_both_some(lhs.track_number(), rhs.track_number());
    let track_length_distance =
        Distance::between_options_if_both_some(lhs.track_length(), rhs.track_length());
    let musicbrainz_recording_id_distance = lhs
        .musicbrainz_recording_id()
        .zip(rhs.musicbrainz_recording_id())
        .map(|(a, b)| string::is_nonempty_and_equal_trimmed(a, b))
        .map(Distance::from);

    [
        Some(track_title_distance),
        track_artist_distance,
        track_number_distance,
        track_length_distance,
        musicbrainz_recording_id_distance,
    ]
    .into_iter()
    .flatten()
    .sum()
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
