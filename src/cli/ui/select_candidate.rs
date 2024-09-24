// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Candidate Selection.

use crate::musicbrainz::MusicBrainzId;
use crate::release::ReleaseLike;
use crate::release_candidate::{ReleaseCandidate, ReleaseCandidateCollection};
use crossterm::style::{Color, Stylize};
use inquire::{validator::Validation, InquireError, Select, Text};
use std::borrow::Cow;
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

impl<T: ReleaseLike> fmt::Display for ReleaseCandidateSelectionOption<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            ReleaseCandidateSelectionOption::Candidate(candidate) => {
                let artist = candidate
                    .release()
                    .release_artist()
                    .unwrap_or_else(|| Cow::from("[unknown artist]".grey().to_string()));
                let album = candidate
                    .release()
                    .release_title()
                    .unwrap_or_else(|| Cow::from("[unknown album]".grey().to_string()));
                let similarity_percent = (1.0 - candidate.distance().weighted_distance()) * 100.0;
                let similarity_color = if similarity_percent >= 90.0 {
                    Color::Green
                } else if similarity_percent >= 50.0 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let similarity = format!("{similarity_percent:.02}")
                    .with(similarity_color)
                    .bold();
                write!(
                    f,
                    "{artist} - {album} {brace_open}{similarity}{brace_close}",
                    brace_open = '('.grey(),
                    brace_close = ')'.grey()
                )
            }
            ReleaseCandidateSelectionOption::EnterMusicBrainzId
            | ReleaseCandidateSelectionOption::SkipItem => {
                let text = match &self {
                    ReleaseCandidateSelectionOption::EnterMusicBrainzId => "Enter MusicBrainz ID",
                    ReleaseCandidateSelectionOption::SkipItem => "Skip Item",
                    ReleaseCandidateSelectionOption::Candidate(_) => unreachable!(),
                };
                write!(f, "{}", text.blue())
            }
        }
    }
}

/// Present a selection of releases to the user, and loop until either a release was selected or
/// the item is skipped. In the latter case, `None` is returned.
pub fn select_candidate<'a, T: ReleaseLike>(
    candidates: &'a ReleaseCandidateCollection<T>,
) -> Result<ReleaseCandidateSelectionResult<'a, T>, InquireError> {
    let additional_options = [
        ReleaseCandidateSelectionOption::EnterMusicBrainzId,
        ReleaseCandidateSelectionOption::SkipItem,
    ];
    let options: Vec<ReleaseCandidateSelectionOption<'a, T>> = candidates
        .iter()
        .map(ReleaseCandidateSelectionOption::Candidate)
        .chain(additional_options)
        .collect();
    loop {
        let prompt = format!("Select one of {} release candidate:", candidates.len());
        let selection = Select::new(&prompt, options.clone()).prompt()?;
        match selection {
            ReleaseCandidateSelectionOption::Candidate(candidate) => {
                break Ok(ReleaseCandidateSelectionResult::Candidate(candidate))
            }
            ReleaseCandidateSelectionOption::SkipItem => {
                break Err(InquireError::OperationCanceled)
            }
            ReleaseCandidateSelectionOption::EnterMusicBrainzId => {
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
        }
    }
}
