// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Release Candidate

use crate::distance::{Distance, ReleaseSimilarity};
use crate::release::ReleaseLike;
use crate::Config;

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
        };
    }

    /// Add a new release to this collection. Create a new candidate internally.
    pub fn add_release<R: ReleaseLike>(&mut self, release: T, base_release: &R, config: &Config) {
        let candidate = ReleaseCandidate::with_base_release(release, base_release, config);
        self.add_candidate(candidate, config);
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
