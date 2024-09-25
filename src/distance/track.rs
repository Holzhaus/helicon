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
use crate::Config;

/// Result of a comparison between two tracks that represents how similar they are to each other.
#[derive(Debug, Clone)]
pub struct TrackSimilarity {
    /// The distance between the two track titles.
    track_title: Distance,
    /// The distance between the two track artists.
    track_artist: Option<Distance>,
    /// The distance between the two track numbers.
    track_number: Option<Distance>,
    /// The distance between the two track lengths.
    track_length: Option<Distance>,
    /// The distance between the two MusicBrainz Recording Ids.
    musicbrainz_recording_id: Option<Distance>,
}

impl TrackSimilarity {
    /// Returns `true` if the track title is equal on both tracks.
    pub fn is_track_title_equal(&self) -> bool {
        self.track_title.is_equality()
    }

    /// Returns `true` if the track artist is equal on both tracks.
    pub fn is_track_artist_equal(&self) -> bool {
        self.track_artist
            .as_ref()
            .is_some_and(Distance::is_equality)
    }

    /// Returns `true` if the track number is equal on both tracks.
    pub fn is_track_number_equal(&self) -> bool {
        self.track_number
            .as_ref()
            .is_some_and(Distance::is_equality)
    }

    /// Returns `true` if the track length is equal on both tracks.
    pub fn is_track_length_equal(&self) -> bool {
        self.track_length
            .as_ref()
            .is_some_and(Distance::is_equality)
    }

    /// Returns `true` if the MusicBrainz Recording ID is equal on both tracks.
    pub fn is_musicbrainz_recording_id_equal(&self) -> bool {
        self.musicbrainz_recording_id
            .as_ref()
            .is_some_and(Distance::is_equality)
    }

    /// Returns the overall distance of the two tracks.
    pub fn total_distance(&self) -> Distance {
        [
            Some(&self.track_title),
            self.track_artist.as_ref(),
            self.track_number.as_ref(),
            self.track_length.as_ref(),
            self.musicbrainz_recording_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        .sum()
    }

    /// Calculate the distance between two releases.
    pub fn detect<T1, T2>(config: &Config, lhs: &T1, rhs: &T2) -> Self
    where
        T1: TrackLike + ?Sized,
        T2: TrackLike + ?Sized,
    {
        let weights = &config.weights.track;

        let track_title = Distance::between_options_or_minmax(lhs.track_title(), rhs.track_title())
            .with_weight(weights.track_title.expect("undefined track_title weight"));
        let track_artist = lhs
            .track_artist()
            .zip(rhs.track_artist())
            .map(Distance::between_tuple_items)
            .map(|distance| {
                distance.with_weight(weights.track_artist.expect("undefined track_artist weight"))
            });
        let track_number = lhs
            .track_number()
            .zip(rhs.track_number())
            .map(Distance::between_tuple_items)
            .map(|distance| {
                distance.with_weight(weights.track_number.expect("undefined track_number weight"))
            });
        let track_length = lhs
            .track_length()
            .zip(rhs.track_length())
            .map(Distance::between_tuple_items)
            .map(|distance| {
                distance.with_weight(weights.track_length.expect("undefined track_length weight"))
            });
        let musicbrainz_recording_id = lhs
            .musicbrainz_recording_id()
            .zip(rhs.musicbrainz_recording_id())
            .map(|(a, b)| string::is_nonempty_and_equal_trimmed(a, b))
            .map(Distance::from)
            .map(|distance| {
                distance.with_weight(
                    weights
                        .musicbrainz_recording_id
                        .expect("undefined musicbrainz_recording_id weight"),
                )
            });

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
        let distance = TrackSimilarity::detect(&config, &track, &track).total_distance();
        assert_float_eq!(distance.weighted_distance(), 0.0, abs <= 0.000_1);
    }

    #[test]
    fn test_track_distance_title_distinct() {
        let track1 = TestTrack("foo");
        let track2 = TestTrack("bar");
        let config = Config::default();
        let distance = TrackSimilarity::detect(&config, &track1, &track2).total_distance();
        assert_float_eq!(distance.weighted_distance(), 1.0, abs <= 0.000_1);
    }

    #[test]
    fn test_track_distance_title_similar() {
        let track1 = TestTrack("foo");
        let track2 = TestTrack("barfoo");
        let config = Config::default();
        let distance = TrackSimilarity::detect(&config, &track1, &track2).total_distance();
        assert_float_eq!(distance.weighted_distance(), 0.5, abs <= 0.000_1);
    }
}
