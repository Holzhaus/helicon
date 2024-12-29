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
use itertools::Itertools;
use std::fmt;
use std::iter;

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
    /// Print the track list.
    PrintTrackList,
    /// Save release information to file (for debugging).
    #[cfg(feature = "dev")]
    DumpReleaseInfo,
    /// Quit (i.e., skip this item and all following).
    Quit,
}

/// An option presented when selecting a release.
enum ReleaseCandidateSelectionOption<'a, T: ReleaseLike> {
    /// Select this release candidate.
    Candidate(&'a ReleaseCandidate<T>),
    /// Enter a customer MusicBrainz release ID.
    EnterMusicBrainzId,
    /// Print the track list.
    PrintTrackList,
    /// DumpReleaseInfo release for debugging.
    #[cfg(feature = "dev")]
    DumpReleaseInfo,
    /// Skip this item.
    SkipItem,
    /// Quit (i.e., skip this item and all following).
    Quit,
}

// Manual implementation of `Clone` to work around unnecessary trait bound `T: Clone`.
impl<T: ReleaseLike> Clone for ReleaseCandidateSelectionOption<'_, T> {
    fn clone(&self) -> Self {
        match &self {
            Self::Candidate(candidate) => Self::Candidate(candidate),
            Self::EnterMusicBrainzId => Self::EnterMusicBrainzId,
            Self::PrintTrackList => Self::PrintTrackList,
            #[cfg(feature = "dev")]
            Self::DumpReleaseInfo => Self::DumpReleaseInfo,
            Self::SkipItem => Self::SkipItem,
            Self::Quit => Self::Quit,
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
        if let ReleaseCandidateSelectionOption::Candidate(candidate) = &self.1 {
            let release_artist_and_title =
                util::format_release_artist_and_title(candidate.release());
            let similarity_percentage = self
                .0
                .user_interface
                .candidate_details
                .candidate_similarity_style
                .apply(util::format_similarity(&candidate.distance(self.0)))
                .to_string();
            let similarity = iter::once(similarity_percentage)
                .chain(
                    candidate
                        .similarity()
                        .problems()
                        .map(|problem| problem.to_string()),
                )
                .join(
                    &self
                        .0
                        .user_interface
                        .candidate_details
                        .candidate_similarity_prefix_style
                        .apply(", ")
                        .to_string(),
                );
            write!(
                f,
                "{release_artist_and_title}{similarity_prefix}{similarity}{similarity_suffix}",
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
        } else {
            let text = match &self.1 {
                ReleaseCandidateSelectionOption::EnterMusicBrainzId => "Enter MusicBrainz ID",
                ReleaseCandidateSelectionOption::SkipItem => "Skip Item",
                ReleaseCandidateSelectionOption::PrintTrackList => "Print Tracklist",
                #[cfg(feature = "dev")]
                ReleaseCandidateSelectionOption::DumpReleaseInfo => "Dump Releases for Debugging",
                ReleaseCandidateSelectionOption::Quit => "Quit",
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
            if best_candidate.distance(config).as_f64() <= 0.05 {
                return Ok(ReleaseCandidateSelectionResult::Candidate(best_candidate));
            }
        }
    }

    let additional_options = [
        ReleaseCandidateSelectionOption::EnterMusicBrainzId,
        ReleaseCandidateSelectionOption::PrintTrackList,
        #[cfg(feature = "dev")]
        ReleaseCandidateSelectionOption::DumpReleaseInfo,
        ReleaseCandidateSelectionOption::SkipItem,
        ReleaseCandidateSelectionOption::Quit,
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
            Ok(ReleaseCandidateSelectionOption::PrintTrackList) => {
                break Ok(ReleaseCandidateSelectionResult::PrintTrackList);
            }
            Ok(ReleaseCandidateSelectionOption::EnterMusicBrainzId) => {
                if let Some(option) = enter_musicbrainz_id() {
                    break Ok(option);
                }
            }
            #[cfg(feature = "dev")]
            Ok(ReleaseCandidateSelectionOption::DumpReleaseInfo) => {
                break Ok(ReleaseCandidateSelectionResult::DumpReleaseInfo);
            }
            Ok(ReleaseCandidateSelectionOption::SkipItem)
            | Err(InquireError::OperationCanceled) => {
                break Ok(ReleaseCandidateSelectionResult::Skipped)
            }
            Ok(ReleaseCandidateSelectionOption::Quit) => {
                break Ok(ReleaseCandidateSelectionResult::Quit)
            }
            Err(err) => Err(err)?,
        }
    }
}

/// Prompt the user to enter a MusicBrainz Release or Release Group ID.
fn enter_musicbrainz_id<'a, T: ReleaseLike>() -> Option<ReleaseCandidateSelectionResult<'a, T>> {
    let result = Text::new("Enter MusicBrainz ID or URL: ")
        .with_validator(validate_musicbrainz_id)
        .prompt();
    if let Ok(text) = result {
        match MusicBrainzId::find(&text) {
            Some(MusicBrainzId::Release(id)) => Some(
                ReleaseCandidateSelectionResult::FetchCandidateRelease(id.to_string()),
            ),
            Some(MusicBrainzId::ReleaseGroup(id)) => Some(
                ReleaseCandidateSelectionResult::FetchCandidateReleaseGroup(id.to_string()),
            ),
            _ => None,
        }
    } else {
        None
    }
}

/// Validator function for MusicBrainz Release and Release Group IDs.
#[expect(clippy::unnecessary_wraps)]
fn validate_musicbrainz_id(
    input: &str,
) -> Result<Validation, Box<dyn std::error::Error + Send + Sync>> {
    if input.is_empty() {
        return Ok(Validation::Valid);
    }
    match MusicBrainzId::find(input) {
        Some(MusicBrainzId::Release(_) | MusicBrainzId::ReleaseGroup(_)) => Ok(Validation::Valid),
        Some(id) => Ok(Validation::Invalid(
            format!(
                "This is a MusicBrainz {} ID, not a release ID.",
                id.entity_name()
            )
            .into(),
        )),
        None => Ok(Validation::Invalid("Not a valid MusicBrainz ID.".into())),
    }
}
