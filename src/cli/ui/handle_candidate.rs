// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Show candidate details and select next action.

use super::util;
use crate::release::ReleaseLike;
use crate::release_candidate::ReleaseCandidate;
use crossterm::style::Stylize;
use inquire::{InquireError, Select};
use std::fmt;

/// The result of a `handle_candidate` all.
pub enum HandleCandidateResult {
    /// Apply the current candidate.
    Apply,
    /// Skip the release.
    Skip,
    /// Back to candidate selection.
    BackToSelection,
}

impl fmt::Display for HandleCandidateResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match &self {
            Self::Apply => "Apply candidate",
            Self::Skip => "Skip album",
            Self::BackToSelection => "Back to candidate selection",
        };
        write!(f, "{}", text.blue())
    }
}

/// Display details about the candidate.
pub fn handle_candidate<T: ReleaseLike>(
    candidate: &ReleaseCandidate<T>,
) -> Result<HandleCandidateResult, InquireError> {
    let distance_color = util::distance_color(&candidate.distance());

    let release_artist_and_title = util::format_release_artist_and_title(candidate.release());
    println!(
        "{release_artist_and_title}",
        release_artist_and_title = release_artist_and_title.with(distance_color).bold()
    );
    println!(
        "Similarity: {similarity}",
        similarity = util::format_similarity(&candidate.distance())
    );

    let options = vec![
        HandleCandidateResult::Apply,
        HandleCandidateResult::Skip,
        HandleCandidateResult::BackToSelection,
    ];

    match Select::new("Select an option:", options).prompt() {
        Ok(option) => Ok(option),
        Err(InquireError::OperationCanceled) => Ok(HandleCandidateResult::BackToSelection),
        result => result,
    }
}
