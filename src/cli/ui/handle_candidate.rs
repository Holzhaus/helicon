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
use crossterm::{style::Stylize, terminal};
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

    let release = candidate.release();
    let release_artist_and_title = util::format_release_artist_and_title(release);
    println!(
        "{release_artist_and_title}",
        release_artist_and_title = release_artist_and_title.with(distance_color).bold()
    );
    println!(
        "Similarity: {similarity}",
        similarity = util::format_similarity(&candidate.distance())
    );

    // Show release metadata
    let max_length = terminal::size().map_or(80, |(cols, _rows)| usize::from(cols));
    let release_meta = [
        release.media_format(),
        release.release_date(),
        release.release_country(),
        release.record_label(),
        release.catalog_number(),
        release.barcode(),
    ]
    .into_iter()
    .flatten()
    .fold(String::new(), |text, item| {
        if text.is_empty() {
            if (text.len() + item.len()) > max_length {
                return text;
            }

            text + item.as_ref()
        } else {
            if (text.len() + item.len() + 3) > max_length {
                return text;
            }

            text + " | " + item.as_ref()
        }
    });
    println!("{}", release_meta.grey());

    if let Some(mb_url) = release.musicbrainz_release_url() {
        println!("{}", mb_url.grey());
    }

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
