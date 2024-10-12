// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Candidate Selection.

use super::util;
use crate::config::Config;
use crate::musicbrainz::MusicBrainzId;
use crate::release::ReleaseLike;
use crate::release_candidate::{ReleaseCandidate, ReleaseCandidateCollection};
use inquire::{validator::Validation, InquireError, Select, Text};
use std::fmt;

/// An option presented when selecting a release.
#[derive(Clone)]
pub enum ReleaseCandidateSelectionResult<'a, T: ReleaseLike> {
    /// Select this release candidate.
    Candidate(&'a ReleaseCandidate<T>),
    /// Fetch a new MusicBrainz release ID and add this as a candidate.
    FetchCandidateRelease(String),
    /// Fetch a new MusicBrainz release group ID and add its releases as a candidates.
    FetchCandidateReleaseGroup(String),
    /// The item was skipped.
    Skipped,
}

/// An option presented when selecting a release.
enum ReleaseCandidateSelectionOption<'a, T: ReleaseLike> {
    /// Select this release candidate.
    Candidate(&'a ReleaseCandidate<T>),
    /// Enter a customer MusicBrainz release ID.
    EnterMusicBrainzId,
    /// Skip this item.
    SkipItem,
}

// Manual implementation of `Clone` to work around unnecessary trait bound `T: Clone`.
impl<T: ReleaseLike> Clone for ReleaseCandidateSelectionOption<'_, T> {
    fn clone(&self) -> Self {
        match &self {
            Self::Candidate(candidate) => Self::Candidate(candidate),
            Self::EnterMusicBrainzId => Self::EnterMusicBrainzId,
            Self::SkipItem => Self::SkipItem,
        }
    }
}

/// A styled version of `ReleaseCandidateSelectionOption` that is displayed to the user.
struct StyledReleaseCandidateSelectionOption<'a, T: ReleaseLike>(
    &'a Config,
    ReleaseCandidateSelectionOption<'a, T>,
);

// Manual implementation of `Clone` to work around unnecessary trait bound `T: Clone`.
impl<T: ReleaseLike> Clone for StyledReleaseCandidateSelectionOption<'_, T> {
    fn clone(&self) -> Self {
        StyledReleaseCandidateSelectionOption(self.0, self.1.clone())
    }
}

impl<T: ReleaseLike> fmt::Display for StyledReleaseCandidateSelectionOption<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.1 {
            ReleaseCandidateSelectionOption::Candidate(candidate) => {
                let release_artist_and_title =
                    util::format_release_artist_and_title(candidate.release());
                let similarity = util::format_similarity(&candidate.distance());
                write!(
                    f,
                    "{release_artist_and_title}{similarity_prefix}{similarity}{similarity_suffix}",
                    similarity = self
                        .0
                        .user_interface
                        .candidate_details
                        .candidate_similarity_style
                        .apply(similarity),
                    similarity_prefix = self
                        .0
                        .user_interface
                        .candidate_details
                        .candidate_similarity_prefix_style
                        .apply(
                            &self
                                .0
                                .user_interface
                                .candidate_details
                                .candidate_similarity_prefix
                        ),
                    similarity_suffix = self
                        .0
                        .user_interface
                        .candidate_details
                        .candidate_similarity_suffix_style
                        .apply(
                            &self
                                .0
                                .user_interface
                                .candidate_details
                                .candidate_similarity_suffix
                        ),
                )
            }
            ReleaseCandidateSelectionOption::EnterMusicBrainzId
            | ReleaseCandidateSelectionOption::SkipItem => {
                let text = match &self.1 {
                    ReleaseCandidateSelectionOption::EnterMusicBrainzId => "Enter MusicBrainz ID",
                    ReleaseCandidateSelectionOption::SkipItem => "Skip Item",
                    ReleaseCandidateSelectionOption::Candidate(_) => unreachable!(),
                };
                write!(
                    f,
                    "{}",
                    self.0
                        .user_interface
                        .candidate_details
                        .action_style
                        .apply(text)
                )
            }
        }
    }
}

impl<'a, T: ReleaseLike> ReleaseCandidateSelectionOption<'a, T> {
    /// Style this `ReleaseCandidateSelectionOption` using the styles defined in the `Config`.
    fn into_styled(self, config: &'a Config) -> StyledReleaseCandidateSelectionOption<'a, T> {
        StyledReleaseCandidateSelectionOption(config, self)
    }
}

impl<'a, T: ReleaseLike> From<StyledReleaseCandidateSelectionOption<'a, T>>
    for ReleaseCandidateSelectionOption<'a, T>
{
    fn from(value: StyledReleaseCandidateSelectionOption<'a, T>) -> Self {
        value.1
    }
}

/// Present a selection of releases to the user, and loop until either a release was selected or
/// the item is skipped. In the latter case, `None` is returned.
pub fn select_candidate<'a, T: ReleaseLike>(
    config: &'a Config,
    candidates: &'a ReleaseCandidateCollection<T>,
    allow_autoselection: bool,
) -> Result<ReleaseCandidateSelectionResult<'a, T>, InquireError> {
    if allow_autoselection {
        if let Some(best_candidate) = candidates.iter().next() {
            if best_candidate.distance().weighted_distance() <= 0.05 {
                return Ok(ReleaseCandidateSelectionResult::Candidate(best_candidate));
            }
        }
    }

    let additional_options = [
        ReleaseCandidateSelectionOption::EnterMusicBrainzId,
        ReleaseCandidateSelectionOption::SkipItem,
    ];
    let options: Vec<StyledReleaseCandidateSelectionOption<'a, T>> = candidates
        .iter()
        .map(ReleaseCandidateSelectionOption::Candidate)
        .chain(additional_options)
        .map(|option| option.into_styled(config))
        .collect();
    loop {
        let prompt = match candidates.len() {
            0 | 1 => "Select release candidate:".to_string(),
            candidate_count => format!("Select one of {candidate_count} release candidates:"),
        };
        match Select::new(&prompt, options.clone())
            .prompt()
            .map(ReleaseCandidateSelectionOption::from)
        {
            Ok(ReleaseCandidateSelectionOption::Candidate(candidate)) => {
                break Ok(ReleaseCandidateSelectionResult::Candidate(candidate))
            }
            Ok(ReleaseCandidateSelectionOption::EnterMusicBrainzId) => {
                let result = Text::new("Enter MusicBrainz ID or URL: ")
                    .with_validator(|input: &str| {
                        if input.is_empty() {
                            return Ok(Validation::Valid);
                        }
                        match MusicBrainzId::find(input) {
                            Some(MusicBrainzId::Release(_) | MusicBrainzId::ReleaseGroup(_)) => {
                                Ok(Validation::Valid)
                            }
                            Some(id) => Ok(Validation::Invalid(
                                format!(
                                    "This is a MusicBrainz {} ID, not a release ID.",
                                    id.entity_name()
                                )
                                .into(),
                            )),
                            None => Ok(Validation::Invalid("Not a valid MusicBrainz ID.".into())),
                        }
                    })
                    .prompt();
                if let Ok(text) = result {
                    match MusicBrainzId::find(&text) {
                        Some(MusicBrainzId::Release(id)) => {
                            break Ok(ReleaseCandidateSelectionResult::FetchCandidateRelease(
                                id.to_string(),
                            ))
                        }
                        Some(MusicBrainzId::ReleaseGroup(id)) => {
                            break Ok(ReleaseCandidateSelectionResult::FetchCandidateReleaseGroup(
                                id.to_string(),
                            ))
                        }
                        _ => (),
                    }
                }
            }
            Ok(ReleaseCandidateSelectionOption::SkipItem)
            | Err(InquireError::OperationCanceled) => {
                break Ok(ReleaseCandidateSelectionResult::Skipped)
            }
            Err(err) => Err(err)?,
        }
    }
}
