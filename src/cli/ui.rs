// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! User Interface (UI) utilities.

use crate::musicbrainz::ReleaseCandidate;
use crate::release::ReleaseLike;
use crossterm::style::{Color, Stylize};
use inquire::{InquireError, Select};
use std::borrow::Cow;
use std::fmt;

/// An option presented when selecting a release.
#[derive(Clone)]
enum ReleaseCandidateSelectionOption<'a> {
    /// Select this release candidate.
    Candidate(&'a ReleaseCandidate),
    /// Skip this item.
    SkipItem,
}

impl fmt::Display for ReleaseCandidateSelectionOption<'_> {
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
            ReleaseCandidateSelectionOption::SkipItem => {
                let text = "Skip Item";
                write!(f, "{}", text.blue())
            }
        }
    }
}

/// Present a selection of releases to the user, and loop until either a release was selected or
/// the item is skipped. In the latter case, `None` is returned.
pub fn select_candidate<'a>(
    candidates: impl Iterator<Item = &'a ReleaseCandidate>,
) -> Result<&'a ReleaseCandidate, InquireError> {
    let additional_options = [ReleaseCandidateSelectionOption::SkipItem];
    let options: Vec<ReleaseCandidateSelectionOption<'a>> = candidates
        .map(ReleaseCandidateSelectionOption::Candidate)
        .chain(additional_options)
        .collect();
    let selection = Select::new("Select a release candidate:", options.clone()).prompt()?;
    match selection {
        ReleaseCandidateSelectionOption::Candidate(candidate) => Ok(candidate),
        ReleaseCandidateSelectionOption::SkipItem => Err(InquireError::OperationCanceled),
    }
}
