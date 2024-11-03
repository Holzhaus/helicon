// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions for distance calculation between [`ReleaseLike`] objects.

use super::{string, Difference, Distance};
use crate::track::TrackLike;
use crate::Config;

/// Result of a comparison between two tracks that represents how similar they are to each other.
#[derive(Debug, Clone)]
pub struct TrackSimilarity {
    /// The distance between the two track titles.
    pub track_title: Difference,
    /// The distance between the two track artists.
    pub track_artist: Difference,
    /// The distance between the two track numbers.
    pub track_number: Difference,
    /// The distance between the two track lengths.
    pub track_length: Difference,
    /// The distance between the two MusicBrainz Recording Ids.
    pub musicbrainz_recording_id: Difference,
}

impl TrackSimilarity {
    /// Returns the overall distance of the two tracks.
    pub fn total_distance(&self, config: &Config) -> Distance {
        let weights = &config.weights.track;

        [
            self.track_title
                .to_distance()
                .clone()
                .with_weight(weights.track_title)
                .into(),
            self.track_artist
                .to_distance_opt()
                .cloned()
                .map(|dist| dist.with_weight(weights.track_artist)),
            self.track_number
                .to_distance_opt()
                .cloned()
                .map(|dist| dist.with_weight(weights.track_number)),
            self.track_length
                .to_distance_opt()
                .cloned()
                .map(|dist| dist.with_weight(weights.track_length)),
            self.musicbrainz_recording_id
                .to_distance_opt()
                .cloned()
                .map(|dist| dist.with_weight(weights.musicbrainz_recording_id)),
        ]
        .iter()
        .flatten()
        .sum()
    }

    /// Calculate the distance between two releases.
    pub fn detect<T1, T2>(lhs: &T1, rhs: &T2) -> Self
    where
        T1: TrackLike + ?Sized,
        T2: TrackLike + ?Sized,
    {
        let track_title = Difference::between_options(lhs.track_title(), rhs.track_title());
        let track_artist = Difference::between_options(lhs.track_artist(), rhs.track_artist());
        let track_number = Difference::between_options(lhs.track_number(), rhs.track_number());
        let track_length = Difference::between_options(lhs.track_length(), rhs.track_length());
        let musicbrainz_recording_id = Difference::between_options_fn(
            lhs.musicbrainz_recording_id(),
            rhs.musicbrainz_recording_id(),
            |lhs, rhs| {
                if string::is_nonempty_and_equal_trimmed(lhs, rhs) {
                    Distance::MIN
                } else {
                    Distance::MAX
                }
            },
        );

        TrackSimilarity {
            track_title,
            track_artist,
            track_number,
            track_length,
            musicbrainz_recording_id,
        }
    }
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
        let config = Config::default();
        let distance = TrackSimilarity::detect(&track, &track).total_distance(&config);
        assert_float_eq!(distance.weighted_distance(), 0.0, abs <= 0.000_1);
    }

    #[test]
    fn test_track_distance_title_distinct() {
        let track1 = TestTrack("foo");
        let track2 = TestTrack("bar");
        let config = Config::default();
        let distance = TrackSimilarity::detect(&track1, &track2).total_distance(&config);
        assert_float_eq!(distance.weighted_distance(), 1.0, abs <= 0.000_1);
    }

    #[test]
    fn test_track_distance_title_similar() {
        let track1 = TestTrack("foo");
        let track2 = TestTrack("barfoo");
        let config = Config::default();
        let distance = TrackSimilarity::detect(&track1, &track2).total_distance(&config);
        assert_float_eq!(distance.weighted_distance(), 0.5, abs <= 0.000_1);
    }
}
