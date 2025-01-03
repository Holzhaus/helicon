// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions for distance calculation between [`ReleaseLike`] objects.

use super::TrackSimilarity;
use super::{string, Difference, Distance, WeightedDistance};
use crate::release::ReleaseLike;
use crate::track::TrackLike;
use crate::Config;
use std::collections::HashMap;
use std::fmt;
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
    #[cfg(test)]
    pub fn new(track_count: usize) -> Self {
        let matched_tracks = (0..track_count)
            .map(|i| TrackMatchPair {
                lhs: i,
                rhs: i,
                similarity: TrackSimilarity::new(),
            })
            .collect();
        TrackAssignment {
            matched_tracks,
            unmatched_tracks: Vec::new(),
            unmatched_tracks_source: UnmatchedTracksSource::Left,
            matched_tracks_distance: Distance::MIN,
        }
    }

    /// Calculates the distance for this track assignment.
    pub fn to_distance(&self) -> Distance {
        let matched_tracks_weight = usize_to_f64(self.matched_tracks.len()).unwrap();
        let unmatched_tracks_weight = usize_to_f64(self.unmatched_tracks.len()).unwrap();
        let matched_tracks_dist = self
            .matched_tracks_distance
            .to_weighted(matched_tracks_weight);
        let unmatched_tracks_dist = Distance::MAX.to_weighted(unmatched_tracks_weight);
        [matched_tracks_dist, unmatched_tracks_dist]
            .into_iter()
            .sum::<Distance>()
    }

    /// Calculates the weighted distance for this track assignment.
    pub fn to_weighted_distance<'a>(&self) -> WeightedDistance<'a> {
        self.to_distance().into_weighted(
            usize_to_f64(self.matched_tracks.len() + self.unmatched_tracks.len()).unwrap(),
        )
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

        if lhs_tracks.is_empty() {
            return TrackAssignment {
                matched_tracks: Vec::new(),
                unmatched_tracks: rhs_tracks.iter().enumerate().map(|(i, _)| i).collect(),
                unmatched_tracks_source: UnmatchedTracksSource::Right,
                matched_tracks_distance: Distance::MAX,
            };
        } else if rhs_tracks.is_empty() {
            return TrackAssignment {
                matched_tracks: Vec::new(),
                unmatched_tracks: lhs_tracks.iter().enumerate().map(|(i, _)| i).collect(),
                unmatched_tracks_source: UnmatchedTracksSource::Left,
                matched_tracks_distance: Distance::MAX,
            };
        }

        let track_similarity_matrix: Vec<TrackSimilarity> = lhs_tracks
            .iter()
            .flat_map(|lhs_track| iter::repeat(lhs_track).zip(rhs_tracks.iter()))
            .map(|(lhs_track, rhs_track)| TrackSimilarity::detect(*lhs_track, *rhs_track))
            .collect();
        let track_distance_matrix_height = lhs_tracks.len(); // number of rows
        let track_distance_matrix_width = rhs_tracks.len(); // number of columns
        let track_distance_matrix: Option<Vec<u64>> = track_similarity_matrix
            .iter()
            .map(|distance| {
                f64_to_u64(
                    (distance.total_distance(config).as_f64() * TRACK_DISTANCE_PRECISION_FACTOR)
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

    /// Returns a [`HashMap`] that allows retrieving the matched tracks from the right hand side
    /// by the corresponding track index from the left hand side.
    pub fn map_lhs_indices_to_rhs(&self) -> HashMap<usize, (usize, &TrackSimilarity)> {
        self.matched_tracks()
            .map(|pair| (pair.lhs, (pair.rhs, &pair.similarity)))
            .collect()
    }

    /// Returns a [`HashMap`] that allows retrieving the matched tracks from the left hand side
    /// by the corresponding track index from the right hand side.
    pub fn map_rhs_indices_to_lhs(&self) -> HashMap<usize, (usize, &TrackSimilarity)> {
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
    release_title: Difference,
    /// The distance between the two release artists.
    release_artist: Difference,
    /// The distance between the two MusicBrainz Release IDs.
    musicbrainz_release_id: Difference,
    /// The distance between the two media formats.
    media_format: Difference,
    /// The distance between the two record labels.
    record_label: Difference,
    /// The distance between the two catalog numbers.
    catalog_number: Difference,
    /// The distance between the two barcodes.
    barcode: Difference,
    /// The minimum distance mapping of tracks from the two releases.
    track_assignment: TrackAssignment,
}

impl ReleaseSimilarity {
    #[cfg(test)]
    pub fn new(track_count: usize) -> Self {
        ReleaseSimilarity {
            release_title: Difference::Added,
            release_artist: Difference::Added,
            musicbrainz_release_id: Difference::Added,
            media_format: Difference::Added,
            record_label: Difference::Added,
            catalog_number: Difference::Added,
            barcode: Difference::Added,
            track_assignment: TrackAssignment::new(track_count),
        }
    }

    /// Calculate the distance between two releases.
    pub fn detect<T1, T2>(config: &Config, lhs: &T1, rhs: &T2) -> Self
    where
        T1: ReleaseLike + ?Sized,
        T2: ReleaseLike + ?Sized,
    {
        let release_title = Difference::between_options(lhs.release_title(), rhs.release_title());
        let release_artist =
            Difference::between_options(lhs.release_artist(), rhs.release_artist());
        let musicbrainz_release_id = Difference::between_options_fn(
            lhs.musicbrainz_release_id(),
            rhs.musicbrainz_release_id(),
            |lhs, rhs| {
                if string::is_nonempty_and_equal_trimmed(lhs, rhs) {
                    Distance::MIN
                } else {
                    Distance::MAX
                }
            },
        );
        let media_format =
            Difference::between_options(lhs.release_media_format(), rhs.release_media_format());
        let record_label = Difference::between_options(lhs.record_label(), rhs.record_label());
        let catalog_number =
            Difference::between_options(lhs.catalog_number(), rhs.catalog_number());
        let barcode = Difference::between_options(lhs.barcode(), rhs.barcode());

        let track_assignment =
            TrackAssignment::compute_from(config, lhs.release_tracks(), rhs.release_tracks());
        Self {
            release_title,
            release_artist,
            musicbrainz_release_id,
            media_format,
            record_label,
            catalog_number,
            barcode,
            track_assignment,
        }
    }

    /// Returns the overall distance of the two releases.
    pub fn total_distance(&self, config: &Config) -> Distance {
        let weights = &config.weights.release;

        let track_assignment_distance = self.track_assignment.to_weighted_distance();
        [
            self.release_title
                .to_distance()
                .to_weighted(weights.release_title)
                .into(),
            self.release_artist
                .to_distance_if_both_present()
                .map(|dist| dist.to_weighted(weights.release_artist)),
            self.musicbrainz_release_id
                .to_distance_if_both_present()
                .map(|dist| dist.to_weighted(weights.musicbrainz_release_id)),
            self.media_format
                .to_distance_if_both_present()
                .map(|dist| dist.to_weighted(weights.media_format)),
            self.record_label
                .to_distance_if_both_present()
                .map(|dist| dist.to_weighted(weights.record_label)),
            self.catalog_number
                .to_distance_if_both_present()
                .map(|dist| dist.to_weighted(weights.catalog_number)),
            self.barcode
                .to_distance_if_both_present()
                .map(|dist| dist.to_weighted(weights.barcode)),
            track_assignment_distance.into(),
        ]
        .into_iter()
        .flatten()
        .sum()
    }

    /// Get a reference to the [`TrackAssignment`] struct.
    pub fn track_assignment(&self) -> &TrackAssignment {
        &self.track_assignment
    }

    /// Returns an iterator over matching problems.
    pub fn problems(&self) -> impl Iterator<Item = SimilarityProblem> + '_ {
        iter::once_with(|| {
            let unmatched_track_count = self.track_assignment().unmatched_tracks().len();
            if unmatched_track_count > 0 {
                return match self.track_assignment().unmatched_tracks_source() {
                    UnmatchedTracksSource::Left => {
                        SimilarityProblem::ResidualTracks(unmatched_track_count).into()
                    }
                    UnmatchedTracksSource::Right => {
                        SimilarityProblem::MissingTracks(unmatched_track_count).into()
                    }
                };
            }

            None
        })
        .chain(iter::once_with(|| {
            if let Difference::BothPresent(dist) = &self.musicbrainz_release_id {
                if !dist.is_equality() {
                    return SimilarityProblem::WrongReleaseId.into();
                }
            }

            None
        }))
        .flatten()
    }
}

/// A problem for the similarity.
#[derive(Debug, Clone, Copy)]
pub enum SimilarityProblem {
    /// There are missing tracks.
    MissingTracks(usize),
    /// There are residual tracks.
    ResidualTracks(usize),
    /// The release ID is present on both releases, but it differs.
    WrongReleaseId,
}

impl fmt::Display for SimilarityProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTracks(count) => write!(f, "{count} missing tracks"),
            Self::ResidualTracks(count) => write!(f, "{count} residual tracks"),
            Self::WrongReleaseId => write!(f, "wrong id"),
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::util::FakeTrack;
    use float_eq::assert_float_eq;

    #[test]
    fn test_track_assignment_exact() {
        let tracks = [
            FakeTrack::with_title("foo"),
            FakeTrack::with_title("bar"),
            FakeTrack::with_title("uvw"),
            FakeTrack::with_title("qrst"),
            FakeTrack::with_title("xyz"),
        ];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, tracks.iter(), tracks.iter());
        assert_eq!(assignment.matched_tracks.len(), 5);
        assert_eq!(assignment.unmatched_tracks.len(), 0);
        assert_float_eq!(
            assignment.to_weighted_distance().as_f64(),
            0.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_shuffled() {
        let lhs = [
            FakeTrack::with_title("foo"),
            FakeTrack::with_title("bar"),
            FakeTrack::with_title("uvw"),
            FakeTrack::with_title("qrst"),
            FakeTrack::with_title("xyz"),
        ];
        let rhs = [
            FakeTrack::with_title("xyz"),
            FakeTrack::with_title("qrst"),
            FakeTrack::with_title("foo"),
            FakeTrack::with_title("bar"),
            FakeTrack::with_title("uvw"),
        ];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 5);
        assert_eq!(assignment.unmatched_tracks.len(), 0);
        assert_float_eq!(
            assignment.to_weighted_distance().as_f64(),
            0.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_distinct() {
        let lhs = [FakeTrack::with_title("foo"), FakeTrack::with_title("bar")];
        let rhs = [FakeTrack::with_title("qrst"), FakeTrack::with_title("xyz")];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 2);
        assert_eq!(assignment.unmatched_tracks.len(), 0);
        assert_float_eq!(assignment.to_distance().as_f64(), 1.0, abs <= 0.000_1);
        assert_float_eq!(
            assignment.to_weighted_distance().as_f64(),
            2.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_lhs_unmatched() {
        let lhs = [
            FakeTrack::with_title("foo"),
            FakeTrack::with_title("bar"),
            FakeTrack::with_title("uvw"),
        ];
        let rhs = [FakeTrack::with_title("qrst"), FakeTrack::with_title("xyz")];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 2);
        assert_eq!(assignment.unmatched_tracks.len(), 1);
        assert_eq!(
            assignment.unmatched_tracks_source,
            UnmatchedTracksSource::Left
        );
        assert_float_eq!(assignment.to_distance().as_f64(), 1.0, abs <= 0.000_1);
        assert_float_eq!(
            assignment.to_weighted_distance().as_f64(),
            3.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_rhs_unmatched() {
        let lhs = [FakeTrack::with_title("foo"), FakeTrack::with_title("bar")];
        let rhs = [
            FakeTrack::with_title("uvw"),
            FakeTrack::with_title("qrst"),
            FakeTrack::with_title("xyz"),
        ];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 2);
        assert_eq!(assignment.unmatched_tracks.len(), 1);
        assert_eq!(
            assignment.unmatched_tracks_source,
            UnmatchedTracksSource::Right
        );
        assert_float_eq!(assignment.to_distance().as_f64(), 1.0, abs <= 0.000_1);
        assert_float_eq!(
            assignment.to_weighted_distance().as_f64(),
            3.0,
            abs <= 0.000_1
        );
    }

    #[test]
    fn test_track_assignment_rhs_empty() {
        let lhs = [FakeTrack::with_title("foo"), FakeTrack::with_title("bar")];
        let rhs: [FakeTrack; 0] = [];

        let config = Config::default();
        let assignment = TrackAssignment::compute_from(&config, lhs.iter(), rhs.iter());
        assert_eq!(assignment.matched_tracks.len(), 0);
        assert_eq!(assignment.unmatched_tracks.len(), 2);
        assert_eq!(
            assignment.unmatched_tracks_source,
            UnmatchedTracksSource::Left
        );
        assert_float_eq!(assignment.to_distance().as_f64(), 1.0, abs <= 0.000_1);
        assert_float_eq!(
            assignment.to_weighted_distance().as_f64(),
            2.0,
            abs <= 0.000_1
        );
    }
}
