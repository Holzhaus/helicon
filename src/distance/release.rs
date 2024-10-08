// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions for distance calculation between [`ReleaseLike`] objects.

use super::TrackSimilarity;
use super::{string, Distance};
use crate::release::ReleaseLike;
use crate::track::TrackLike;
use crate::Config;
use std::cmp;
use std::collections::HashMap;
use std::iter;

/// Convert an `f64` into an `u64`.
///
/// This will only return a value if the f64 is a positive finite value without a fractional part.
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
fn f64_to_u64(value: f64) -> Option<u64> {
    if value.is_finite() && value.is_sign_positive() && value.fract() == 0.0 {
        Some(value.trunc() as u64)
    } else {
        None
    }
}

/// Convert an `u64` value into an `f64` (if possible).
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
fn u64_to_f64(value: u64) -> Option<f64> {
    (value < (f64::MAX.trunc() as u64)).then_some(value as f64)
}

/// Convert an `usize` value into an `f64` (if possible).
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> Option<f64> {
    (value < (f64::MAX.trunc() as usize)).then_some(value as f64)
}

/// The source of the unmatched tracks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnmatchedTracksSource {
    /// The unmatched tracks belong to the left iterator of track items.
    Left,
    /// The unmatched tracks belong to the right iterator of track items.
    Right,
}

/// A pair of tracks that are part of a [`TrackAssignment`].
#[derive(Debug, Clone)]
pub struct TrackMatchPair {
    /// The index of the left track.
    pub lhs: usize,
    /// The index of the right track.
    pub rhs: usize,
    /// The similarity of the two tracks.
    pub similarity: TrackSimilarity,
}

/// Represents a potential assignment between to collections of tracks.
#[derive(Debug, Clone)]
pub struct TrackAssignment {
    /// The assignment of tracks as `Vec` for index pairs.
    matched_tracks: Vec<TrackMatchPair>,
    /// The unmatched tracks as Vec<
    unmatched_tracks: Vec<usize>,
    /// The source of the unmatched tracks.
    #[allow(dead_code)]
    unmatched_tracks_source: UnmatchedTracksSource,
    /// The distance between the matched tracks (excluding the unmatched ones).
    matched_tracks_distance: Distance,
}

impl TrackAssignment {
    /// Calculates the distance for this track assignment.
    pub fn as_distance(&self) -> Distance {
        let matched_tracks_weight = usize_to_f64(self.matched_tracks.len()).unwrap();
        let unmatched_tracks_weight = usize_to_f64(self.unmatched_tracks.len()).unwrap();
        let matched_tracks_dist = self
            .matched_tracks_distance
            .clone()
            .with_weight(matched_tracks_weight);
        let unmatched_tracks_dist = Distance::from(1.0).with_weight(unmatched_tracks_weight);
        [matched_tracks_dist, unmatched_tracks_dist]
            .into_iter()
            .sum::<Distance>()
            .with_weight(matched_tracks_weight + unmatched_tracks_weight)
    }

    /// Compute the best match between two Iterators of [`TrackLike`] items and returns a
    /// [`TrackAssignment`] struct.
    pub fn compute_from<'a>(
        config: &Config,
        lhs: impl Iterator<Item = &'a (impl TrackLike + 'a)>,
        rhs: impl Iterator<Item = &'a (impl TrackLike + 'a)>,
    ) -> TrackAssignment {
        /// Since the `hungarian` crate operates on integers, we'll normalize the [`f64`] distances by
        /// multiplying them with this constant and truncating them, then divide by this constant
        /// afterwards.
        const TRACK_DISTANCE_PRECISION_FACTOR: f64 = 100_000.0;

        let lhs_tracks: Vec<_> = lhs.collect();
        let rhs_tracks: Vec<_> = rhs.collect();

        let track_similarity_matrix: Vec<TrackSimilarity> = lhs_tracks
            .iter()
            .flat_map(|lhs_track| iter::repeat(lhs_track).zip(rhs_tracks.iter()))
            .map(|(lhs_track, rhs_track)| TrackSimilarity::detect(config, *lhs_track, *rhs_track))
            .collect();
        let track_distance_matrix_height = lhs_tracks.len(); // number of rows
        let track_distance_matrix_width = rhs_tracks.len(); // number of columns
        let track_distance_matrix: Option<Vec<u64>> = track_similarity_matrix
            .iter()
            .map(|distance| {
                f64_to_u64(
                    (distance.total_distance().weighted_distance()
                        * TRACK_DISTANCE_PRECISION_FACTOR)
                        .trunc(),
                )
            })
            .collect();
        let track_distance_matrix = track_distance_matrix.unwrap();
        debug_assert_eq!(
            track_distance_matrix_height * track_distance_matrix_width,
            track_distance_matrix.len()
        );

        // Returns a Vec of with `track_distance_matrix_height` items.
        let assignment = hungarian::minimize(
            &track_distance_matrix,
            track_distance_matrix_height,
            track_distance_matrix_width,
        );
        debug_assert_eq!(track_distance_matrix_height, assignment.len());
        debug_assert!(
            assignment.iter().all(Option::is_some)
                || track_distance_matrix_width < track_distance_matrix_height
        );

        // Calculate the matching code.
        let matched_tracks_cost: f64 = assignment
            .iter()
            .enumerate()
            .filter_map(|(i, &opt)| {
                opt.map(|j| track_distance_matrix[i * track_distance_matrix_width + j])
            })
            .map(|integer_dist| {
                let value = u64_to_f64(integer_dist).unwrap() / TRACK_DISTANCE_PRECISION_FACTOR;
                debug_assert!(value.is_finite());
                debug_assert!(value >= 0.0);
                debug_assert!(value <= 1.0);
                value
            })
            .sum();

        // Calculate the resulting number of unmatched tracks and whether they belong to the left or
        // right hand side.
        let (unmatched_track_count, unmatched_tracks_source) =
            if track_distance_matrix_width < track_distance_matrix_height {
                // If there are more rows (lhs) than columns (rhs), then some tracks from the lhs
                // may be unassigned.
                (
                    track_distance_matrix_height - track_distance_matrix_width,
                    UnmatchedTracksSource::Left,
                )
            } else {
                // If there are more columns (rhs) than rows (lhs), then some tracks on the rhs may
                // be unassigned.
                (
                    track_distance_matrix_width - track_distance_matrix_height,
                    UnmatchedTracksSource::Right,
                )
            };
        debug_assert_eq!(
            unmatched_track_count,
            track_distance_matrix_width.max(track_distance_matrix_height)
                - track_distance_matrix_width.min(track_distance_matrix_height)
        );

        let mut matched_tracks =
            Vec::with_capacity(track_distance_matrix_width.min(track_distance_matrix_height));
        let mut unmatched_tracks = Vec::with_capacity(unmatched_track_count);
        assignment
            .into_iter()
            .enumerate()
            .for_each(|pair| match pair {
                (i, Some(j)) => {
                    let matched_track = TrackMatchPair {
                        lhs: i,
                        rhs: j,
                        similarity: track_similarity_matrix[i * track_distance_matrix_width + j]
                            .clone(),
                    };
                    matched_tracks.push(matched_track);
                }
                (i, None) => {
                    debug_assert_eq!(unmatched_tracks_source, UnmatchedTracksSource::Left);
                    unmatched_tracks.push(i);
                }
            });
        if unmatched_tracks_source == UnmatchedTracksSource::Right {
            (0..rhs_tracks.len())
                .filter(|&j| matched_tracks.iter().all(|matched| j != matched.rhs))
                .for_each(|j| unmatched_tracks.push(j));
        }
        debug_assert_eq!(unmatched_track_count, unmatched_tracks.len());

        let matched_tracks_base_distance =
            matched_tracks_cost / usize_to_f64(matched_tracks.len()).unwrap();
        debug_assert!(matched_tracks_base_distance >= 0.0);
        debug_assert!(matched_tracks_base_distance <= 1.0);
        let matched_tracks_distance = Distance::from(matched_tracks_base_distance);

        TrackAssignment {
            matched_tracks,
            unmatched_tracks,
            unmatched_tracks_source,
            matched_tracks_distance,
        }
    }

    /// Returns an iterator over [`TrackMatchPair`] items.
    pub fn matched_tracks(&self) -> impl Iterator<Item = &TrackMatchPair> {
        self.matched_tracks.iter()
    }

    /// Returns a [`HashMap`] that maps the matched tracks from the left hand side side to the
    /// corresponding track on the right hand side.
    pub fn matched_tracks_map(&self) -> HashMap<usize, (usize, &TrackSimilarity)> {
        self.matched_tracks()
            .map(|pair| (pair.rhs, (pair.lhs, &pair.similarity)))
            .collect()
    }

    /// Returns a slice of unmatched track indices. The indices either belong to the left or right
    /// hand side release, depending on the output of UnmatchedTracksSource.
    pub fn unmatched_tracks(&self) -> &[usize] {
        &self.unmatched_tracks
    }

    /// Indicates if the unmatched tracks belong to the left or right hand side release.
    pub fn unmatched_tracks_source(&self) -> UnmatchedTracksSource {
        self.unmatched_tracks_source
    }
}

/// Result of a comparison between two releases that represents how similar they are to each other.
#[derive(Debug, Clone)]
pub struct ReleaseSimilarity {
    /// The distance between the two release titles.
    release_title_distance: Distance,
    /// The distance between the two release artists.
    release_artist_distance: Option<Distance>,
    /// The distance between the two MusicBrainz Release IDs.
    musicbrainz_release_id_distance: Option<Distance>,
    /// The distance between the two media formats.
    media_format_distance: Option<Distance>,
    /// The distance between the two record labels.
    record_label_distance: Option<Distance>,
    /// The distance between the two catalog numbers.
    catalog_number_distance: Option<Distance>,
    /// The distance between the two barcodes.
    barcode_distance: Option<Distance>,
    /// The minimum distance mapping of tracks from the two releases.
    track_assignment: TrackAssignment,
}

impl PartialEq for ReleaseSimilarity {
    fn eq(&self, other: &Self) -> bool {
        self.total_distance().eq(&other.total_distance())
    }
}

impl Eq for ReleaseSimilarity {}

impl PartialOrd for ReleaseSimilarity {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReleaseSimilarity {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.total_distance().cmp(&other.total_distance())
    }
}

impl ReleaseSimilarity {
    /// Calculate the distance between two releases.
    pub fn detect<T1, T2>(config: &Config, lhs: &T1, rhs: &T2) -> Self
    where
        T1: ReleaseLike + ?Sized,
        T2: ReleaseLike + ?Sized,
    {
        let weights = &config.weights.release;

        let release_title_distance =
            Distance::between_options_or_minmax(lhs.release_title(), rhs.release_title())
                .with_weight(weights.release_title);
        let release_artist_distance = lhs
            .release_artist()
            .zip(rhs.release_artist())
            .map(Distance::between_tuple_items)
            .map(|distance| distance.with_weight(weights.release_artist));
        let musicbrainz_release_id_distance = lhs
            .musicbrainz_release_id()
            .zip(rhs.musicbrainz_release_id())
            .map(|(a, b)| string::is_nonempty_and_equal_trimmed(a, b))
            .map(Distance::from)
            .map(|distance| distance.with_weight(weights.musicbrainz_release_id));
        let media_format_distance = lhs
            .release_media_format()
            .zip(rhs.release_media_format())
            .map(Distance::between_tuple_items)
            .map(|distance| distance.with_weight(weights.media_format));
        let record_label_distance = lhs
            .record_label()
            .zip(rhs.record_label())
            .map(Distance::between_tuple_items)
            .map(|distance| distance.with_weight(weights.record_label));
        let catalog_number_distance = lhs
            .catalog_number()
            .zip(rhs.catalog_number())
            .map(Distance::between_tuple_items)
            .map(|distance| distance.with_weight(weights.catalog_number));
        let barcode_distance = lhs
            .barcode()
            .zip(rhs.barcode())
            .map(Distance::between_tuple_items)
            .map(|distance| distance.with_weight(weights.barcode));

        let track_assignment =
            TrackAssignment::compute_from(config, lhs.release_tracks(), rhs.release_tracks());
        Self {
            release_title_distance,
            release_artist_distance,
            musicbrainz_release_id_distance,
            media_format_distance,
            record_label_distance,
            catalog_number_distance,
            barcode_distance,
            track_assignment,
        }
    }

    /// Returns the overall distance of the two releases.
    pub fn total_distance(&self) -> Distance {
        let track_assignment_distance = self.track_assignment.as_distance();
        [
            Some(&self.release_title_distance),
            self.release_artist_distance.as_ref(),
            self.musicbrainz_release_id_distance.as_ref(),
            self.media_format_distance.as_ref(),
            self.record_label_distance.as_ref(),
            self.catalog_number_distance.as_ref(),
            self.barcode_distance.as_ref(),
            Some(&track_assignment_distance),
        ]
        .into_iter()
        .flatten()
        .sum()
    }

    /// Get a reference to the [`TrackAssignment`] struct.
    pub fn track_assignment(&self) -> &TrackAssignment {
        &self.track_assignment
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::super::tests::TestTrack;
    use super::*;
    use float_eq::assert_float_eq;

    #[test]
    fn test_track_assignment_exact() {
        let tracks = [
            TestTrack("foo"),
            TestTrack("bar"),
            TestTrack("uvw"),
            TestTrack("qrst"),
            TestTrack("xyz"),
        ];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, tracks.iter(), tracks.iter());
        assert_eq!(assignment.matched_tracks.len(), 5);
        assert_eq!(assignment.unmatched_tracks.len(), 0);
        assert_float_eq!(
            assignment.as_distance().weighted_distance(),
            0.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_shuffled() {
        let lhs = [
            TestTrack("foo"),
            TestTrack("bar"),
            TestTrack("uvw"),
            TestTrack("qrst"),
            TestTrack("xyz"),
        ];
        let rhs = [
            TestTrack("xyz"),
            TestTrack("qrst"),
            TestTrack("foo"),
            TestTrack("bar"),
            TestTrack("uvw"),
        ];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 5);
        assert_eq!(assignment.unmatched_tracks.len(), 0);
        assert_float_eq!(
            assignment.as_distance().weighted_distance(),
            0.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_distinct() {
        let lhs = [TestTrack("foo"), TestTrack("bar")];
        let rhs = [TestTrack("qrst"), TestTrack("xyz")];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 2);
        assert_eq!(assignment.unmatched_tracks.len(), 0);
        assert_float_eq!(assignment.as_distance().base_distance, 1.0, abs <= 0.000_1);
        assert_float_eq!(
            assignment.as_distance().weighted_distance(),
            2.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_lhs_unmatched() {
        let lhs = [TestTrack("foo"), TestTrack("bar"), TestTrack("uvw")];
        let rhs = [TestTrack("qrst"), TestTrack("xyz")];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 2);
        assert_eq!(assignment.unmatched_tracks.len(), 1);
        assert_eq!(
            assignment.unmatched_tracks_source,
            UnmatchedTracksSource::Left
        );
        assert_float_eq!(assignment.as_distance().base_distance, 1.0, abs <= 0.000_1);
        assert_float_eq!(
            assignment.as_distance().weighted_distance(),
            3.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_rhs_unmatched() {
        let lhs = [TestTrack("foo"), TestTrack("bar")];
        let rhs = [TestTrack("uvw"), TestTrack("qrst"), TestTrack("xyz")];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 2);
        assert_eq!(assignment.unmatched_tracks.len(), 1);
        assert_eq!(
            assignment.unmatched_tracks_source,
            UnmatchedTracksSource::Right
        );
        assert_float_eq!(assignment.as_distance().base_distance, 1.0, abs <= 0.000_1);
        assert_float_eq!(
            assignment.as_distance().weighted_distance(),
            3.0,
            abs <= 0.000_1
        );
    }
}
