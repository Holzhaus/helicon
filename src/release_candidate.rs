// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Release Candidate

use crate::distance::{Distance, ReleaseSimilarity, SimilarityProblem};
use crate::release::ReleaseLike;
use crate::Config;
use serde::{Deserialize, Serialize};

/// How strongly the best release candidate is recommended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    /// No conclusion can be drawn.
    None,
    /// Low recommendation.
    Low,
    /// Medium recommendation.
    Medium,
    /// Strong recommendation.
    Strong,
}

/// A candidate release that potentially matches the base release.
#[derive(Debug, Clone)]
pub struct ReleaseCandidate<T: ReleaseLike> {
    /// The release from MusicBrainz.
    release: T,
    /// The similarity to the base release.
    similarity: ReleaseSimilarity,
}

impl<T: ReleaseLike> ReleaseCandidate<T> {
    /// Create a new candidate from a musicbrainz release and a precalculated similarity to the
    /// base release.
    pub fn with_similarity(release: T, similarity: ReleaseSimilarity) -> Self {
        Self {
            release,
            similarity,
        }
    }

    /// Create a new candidate from a musicbrainz release and compute it's similarity to the base
    /// release on the fly.
    pub fn with_base_release<S: ReleaseLike>(
        release: T,
        base_release: &S,
        config: &Config,
    ) -> Self {
        let similarity = base_release.similarity_to(&release, config);
        Self::with_similarity(release, similarity)
    }

    /// Get a reference to the inner release,
    pub fn release(&self) -> &T {
        &self.release
    }

    /// Get a reference to the similarity struct.;
    pub fn similarity(&self) -> &ReleaseSimilarity {
        &self.similarity
    }

    /// Get the distance to the base release.
    pub fn distance(&self, config: &Config) -> Distance {
        self.similarity.total_distance(config)
    }
}

/// A collection of release candidates.
///
/// Has convenience methods to add new candidates in sorted order.
#[derive(Debug, Clone, Default)]
pub struct ReleaseCandidateCollection<T: ReleaseLike> {
    /// Ordered list of candidates.
    candidates: Vec<ReleaseCandidate<T>>,
}

impl<T: ReleaseLike> ReleaseCandidateCollection<T> {
    /// Find the index of the candidate.
    pub fn find_index(&self, selected_candidate: &ReleaseCandidate<T>) -> usize {
        self.candidates
            .iter()
            .enumerate()
            .find_map(|(i, candidate)| {
                (candidate.release().musicbrainz_release_id()
                    == selected_candidate.release().musicbrainz_release_id())
                .then_some(i)
            })
            .expect("Failed to find selected candidate in candidate collection.")
    }

    /// Select the candidate by index and discard the other candidates.
    pub fn select_index(mut self, index: usize) -> ReleaseCandidate<T> {
        self.candidates.swap_remove(index)
    }

    /// Add a new candidate to this collection.
    pub fn add_candidate(&mut self, candidate: ReleaseCandidate<T>, config: &Config) {
        match self
            .candidates
            .binary_search_by(|cand| cand.distance(config).cmp(&candidate.distance(config)))
        {
            Ok(pos) => {
                // There already is a candidate with the same distance in the candidate list.
                if !self.candidates.iter().skip(pos).any(|c| {
                    c.release().musicbrainz_release_id()
                        == candidate.release().musicbrainz_release_id()
                }) {
                    self.candidates.insert(pos, candidate);
                }
            }
            Err(pos) => {
                log::debug!(
                    "Adding candidate: {}",
                    candidate.release().release_title().unwrap_or_default()
                );
                self.candidates.insert(pos, candidate);
            }
        }
    }

    /// Add a new release to this collection. Create a new candidate internally.
    pub fn add_release<R: ReleaseLike>(&mut self, release: T, base_release: &R, config: &Config) {
        let candidate = ReleaseCandidate::with_base_release(release, base_release, config);
        self.add_candidate(candidate, config);
    }

    /// Determine the recommendation level based on the distances of the best candidates, capped
    /// by the limits configured for the problems of the best candidate.
    pub fn recommendation(&self, config: &Config) -> Recommendation {
        let Some(best) = self.candidates.first() else {
            return Recommendation::None;
        };
        let limits = &config.recommendation.limit;
        best.similarity()
            .problems()
            .map(|problem| match problem {
                SimilarityProblem::MissingTracks(_) => limits.missing_tracks,
                SimilarityProblem::ResidualTracks(_) => limits.residual_tracks,
                SimilarityProblem::WrongReleaseId => limits.wrong_release_id,
            })
            .fold(self.similarity_recommendation(config), Recommendation::min)
    }

    /// Determine the recommendation level from the candidate distances only.
    fn similarity_recommendation(&self, config: &Config) -> Recommendation {
        let thresholds = &config.recommendation.similarity;
        let Some(best) = self.candidates.first() else {
            return Recommendation::None;
        };
        let min_dist = best.distance(config).as_f64();
        if min_dist < thresholds.strong {
            Recommendation::Strong
        } else if min_dist <= thresholds.medium {
            Recommendation::Medium
        } else if let Some(second) = self.candidates.get(1) {
            if second.distance(config).as_f64() - min_dist >= thresholds.gap {
                Recommendation::Low
            } else {
                Recommendation::None
            }
        } else {
            Recommendation::Low
        }
    }

    /// Iterate over the candidates in this collection.
    pub fn iter(&self) -> std::slice::Iter<'_, ReleaseCandidate<T>> {
        self.candidates.iter()
    }

    /// Return the number of the candidates in this collection.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }
}

impl<T: ReleaseLike> From<Vec<ReleaseCandidate<T>>> for ReleaseCandidateCollection<T> {
    /// Create a new release candidate collections.
    ///
    /// The supplied candidates needs to be in the correct order.
    fn from(candidates: Vec<ReleaseCandidate<T>>) -> Self {
        Self { candidates }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        distance::Distance,
        release_candidate::{Recommendation, ReleaseCandidate, ReleaseCandidateCollection},
        util::FakeRelease,
        Config,
    };

    const RELEASE_DATA: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/debug/tuxedo/release.json"
    ));
    const RELEASE_CANDIDATE_0_DATA: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/debug/tuxedo/candidate_0.json"
    ));

    #[test]
    fn test_track_assignment_exact() {
        let release: FakeRelease = serde_json::from_slice(RELEASE_DATA).unwrap();
        let candidate_0: FakeRelease = serde_json::from_slice(RELEASE_CANDIDATE_0_DATA).unwrap();

        let config = Config::default();
        let candidates = ReleaseCandidateCollection::from(
            [candidate_0]
                .into_iter()
                .map(|candidate| ReleaseCandidate::with_base_release(candidate, &release, &config))
                .collect::<Vec<_>>(),
        );
        let distances = candidates
            .iter()
            .map(|candidate| candidate.similarity().total_distance(&config))
            .collect::<Vec<_>>();
        assert_eq!(distances, [Distance::MIN]);
    }

    #[test]
    fn test_recommendation() {
        let release: FakeRelease = serde_json::from_slice(RELEASE_DATA).unwrap();
        let candidate_0: FakeRelease = serde_json::from_slice(RELEASE_CANDIDATE_0_DATA).unwrap();
        let mut config = Config::default();
        let collection =
            ReleaseCandidateCollection::from(vec![ReleaseCandidate::with_base_release(
                candidate_0,
                &release,
                &config,
            )]);
        assert_eq!(collection.recommendation(&config), Recommendation::Strong);

        // Distance 0.0 is not below a threshold of 0.0, but within the medium threshold.
        config.recommendation.similarity.strong = 0.0;
        assert_eq!(collection.recommendation(&config), Recommendation::Medium);

        // A single candidate that exceeds all thresholds is a low recommendation.
        config.recommendation.similarity.medium = -1.0;
        assert_eq!(collection.recommendation(&config), Recommendation::Low);

        let empty = ReleaseCandidateCollection::<FakeRelease>::default();
        assert_eq!(empty.recommendation(&config), Recommendation::None);
    }

    #[test]
    fn test_recommendation_limited_by_problems() {
        let candidate: FakeRelease = serde_json::from_slice(RELEASE_CANDIDATE_0_DATA).unwrap();
        let mut base: serde_json::Value = serde_json::from_slice(RELEASE_DATA).unwrap();
        // Remove a track from the base release, so that the candidate has a missing track.
        base["media"][0]["media_tracks"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        let base: FakeRelease = serde_json::from_value(base).unwrap();

        let mut config = Config::default();
        config.recommendation.similarity.strong = 1.1;
        let collection =
            ReleaseCandidateCollection::from(vec![ReleaseCandidate::with_base_release(
                candidate, &base, &config,
            )]);
        assert_eq!(collection.recommendation(&config), Recommendation::Medium);

        config.recommendation.limit.missing_tracks = Recommendation::Strong;
        assert_eq!(collection.recommendation(&config), Recommendation::Strong);
    }
}
