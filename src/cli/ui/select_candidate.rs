// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
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

use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct DisambiguationNeeds: u8 {
        const RELEASE_YEAR = 1;
        const MEDIA_TYPE = 1 << 1;
        const COUNTRY = 1 << 2;
        const RECORD_LABEL = 1 << 3;
        const CATALOG_NUMBER = 1 << 4;
        const BARCODE = 1 << 5;
    }
}

/// Get a tuple of distinguishing information for a candidate, based on what needs to be shown.
#[expect(clippy::type_complexity)]
fn get_distinguishing_tuple<T: ReleaseLike>(
    candidate: &ReleaseCandidate<T>,
    needs: DisambiguationNeeds,
) -> (
    Option<Cow<'_, str>>,
    Option<Cow<'_, str>>,
    Option<Cow<'_, str>>,
    Option<Cow<'_, str>>,
    Option<Cow<'_, str>>,
    Option<Cow<'_, str>>,
) {
    (
        candidate
            .release()
            .release_year()
            .filter(|_| needs.contains(DisambiguationNeeds::RELEASE_YEAR)),
        candidate
            .release()
            .release_media_format()
            .filter(|_| needs.contains(DisambiguationNeeds::MEDIA_TYPE)),
        candidate
            .release()
            .release_country()
            .filter(|_| needs.contains(DisambiguationNeeds::COUNTRY)),
        candidate
            .release()
            .record_label()
            .filter(|_| needs.contains(DisambiguationNeeds::RECORD_LABEL)),
        candidate
            .release()
            .catalog_number()
            .filter(|_| needs.contains(DisambiguationNeeds::CATALOG_NUMBER)),
        candidate
            .release()
            .barcode()
            .filter(|_| needs.contains(DisambiguationNeeds::BARCODE)),
    )
}

/// Compute which fields need disambiguation for candidates with duplicate artist/title combinations.
fn compute_disambiguation_needs<T: ReleaseLike>(
    candidates: &ReleaseCandidateCollection<T>,
) -> Vec<DisambiguationNeeds> {
    use std::collections::HashMap;

    let candidate_list: Vec<_> = candidates.iter().collect();
    let mut result = vec![DisambiguationNeeds::empty(); candidate_list.len()];

    // Group candidates by (artist, title)
    #[expect(clippy::type_complexity)]
    let mut grouped: HashMap<(Option<Cow<'_, str>>, Option<Cow<'_, str>>), Vec<usize>> =
        HashMap::new();
    for (idx, candidate) in candidate_list.iter().enumerate() {
        let artist_title = (
            candidate.release().release_artist(),
            candidate.release().release_title(),
        );
        grouped.entry(artist_title).or_default().push(idx);
    }

    // For groups with duplicates, determine which fields need disambiguation
    for indices in grouped.values() {
        if indices.len() > 1 {
            let mut needs = DisambiguationNeeds::empty();

            // Try progressively more fields until all candidates are distinguishable
            let fields_to_try = [
                DisambiguationNeeds::MEDIA_TYPE,
                DisambiguationNeeds::COUNTRY,
                DisambiguationNeeds::RELEASE_YEAR,
                DisambiguationNeeds::MEDIA_TYPE | DisambiguationNeeds::COUNTRY,
                DisambiguationNeeds::MEDIA_TYPE | DisambiguationNeeds::RELEASE_YEAR,
                DisambiguationNeeds::COUNTRY | DisambiguationNeeds::RELEASE_YEAR,
                DisambiguationNeeds::MEDIA_TYPE
                    | DisambiguationNeeds::COUNTRY
                    | DisambiguationNeeds::RELEASE_YEAR,
                DisambiguationNeeds::RECORD_LABEL,
                DisambiguationNeeds::RECORD_LABEL | DisambiguationNeeds::MEDIA_TYPE,
                DisambiguationNeeds::RECORD_LABEL | DisambiguationNeeds::CATALOG_NUMBER,
                DisambiguationNeeds::RECORD_LABEL | DisambiguationNeeds::MEDIA_TYPE,
                DisambiguationNeeds::RECORD_LABEL | DisambiguationNeeds::RELEASE_YEAR,
                DisambiguationNeeds::BARCODE,
                DisambiguationNeeds::all(),
            ];

            for test_needs in fields_to_try {
                // Check if all candidates are distinguishable with these fields
                let mut seen = std::collections::HashSet::new();
                let mut all_unique = true;

                for &idx in indices {
                    let tuple = get_distinguishing_tuple(candidate_list[idx], test_needs);
                    if !seen.insert(tuple) {
                        all_unique = false;
                        break;
                    }
                }

                if all_unique {
                    needs = test_needs;
                    break;
                }
            }

            if needs.is_empty() {
                needs = DisambiguationNeeds::all();
            }

            for &idx in indices {
                result[idx] = needs;
            }
        }
    }

    result
}

/// A styled version of `ReleaseCandidateSelectionOption` that is displayed to the user.
struct StyledReleaseCandidateSelectionOption<'a, T: ReleaseLike> {
    /// Configuration settings (needed for custom styling)
    config: &'a Config,
    /// The actual selection option
    option: ReleaseCandidateSelectionOption<'a, T>,
    /// Which fields need to be shown to distinguish it from other candidates
    disambiguation_needs: DisambiguationNeeds,
}

// Manual implementation of `Clone` to work around unnecessary trait bound `T: Clone`.
impl<T: ReleaseLike> Clone for StyledReleaseCandidateSelectionOption<'_, T> {
    fn clone(&self) -> Self {
        StyledReleaseCandidateSelectionOption {
            config: self.config,
            option: self.option.clone(),
            disambiguation_needs: self.disambiguation_needs,
        }
    }
}

impl<T: ReleaseLike> fmt::Display for StyledReleaseCandidateSelectionOption<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let ReleaseCandidateSelectionOption::Candidate(candidate) = &self.option {
            let release_artist_and_title =
                util::format_release_artist_and_title(candidate.release());
            let similarity_percentage = self
                .config
                .user_interface
                .candidate_details
                .candidate_similarity_style
                .apply(Cow::from(util::format_similarity(
                    &candidate.distance(self.config),
                )));

            // Build disambiguation info, filtering based on what needs to be shown
            let mut disambiguation_parts = Vec::new();

            // Media format and count
            if self
                .disambiguation_needs
                .contains(DisambiguationNeeds::MEDIA_TYPE)
            {
                if let Some(media_format) = candidate.release().release_media_format() {
                    let media_count = candidate.release().media().count();
                    let disambig = if media_count > 1 {
                        Cow::from(format!("{media_count}x{media_format}"))
                    } else {
                        media_format
                    };
                    disambiguation_parts.push(
                        self.config
                            .user_interface
                            .candidate_details
                            .candidate_disambiguation_style
                            .apply(disambig),
                    );
                }
            }

            // Release year
            if self
                .disambiguation_needs
                .contains(DisambiguationNeeds::RELEASE_YEAR)
            {
                if let Some(year) = candidate.release().release_year() {
                    disambiguation_parts.push(
                        self.config
                            .user_interface
                            .candidate_details
                            .candidate_disambiguation_style
                            .apply(year),
                    );
                }
            }

            // Country
            if self
                .disambiguation_needs
                .contains(DisambiguationNeeds::COUNTRY)
            {
                if let Some(country) = candidate.release().release_country() {
                    disambiguation_parts.push(
                        self.config
                            .user_interface
                            .candidate_details
                            .candidate_disambiguation_style
                            .apply(country),
                    );
                }
            }

            // Record RECORD_LABEL
            if self
                .disambiguation_needs
                .contains(DisambiguationNeeds::RECORD_LABEL)
            {
                if let Some(record_label) = candidate.release().record_label() {
                    disambiguation_parts.push(
                        self.config
                            .user_interface
                            .candidate_details
                            .candidate_disambiguation_style
                            .apply(record_label),
                    );
                }
            }

            // Catalog Number
            if self
                .disambiguation_needs
                .contains(DisambiguationNeeds::CATALOG_NUMBER)
            {
                if let Some(catalog_number) = candidate.release().catalog_number() {
                    disambiguation_parts.push(
                        self.config
                            .user_interface
                            .candidate_details
                            .candidate_disambiguation_style
                            .apply(catalog_number),
                    );
                }
            }

            // Barcode
            if self
                .disambiguation_needs
                .contains(DisambiguationNeeds::BARCODE)
            {
                if let Some(barcode) = candidate.release().barcode() {
                    disambiguation_parts.push(
                        self.config
                            .user_interface
                            .candidate_details
                            .candidate_disambiguation_style
                            .apply(barcode),
                    );
                }
            }

            // Problems (always show)
            let problems = candidate.similarity().problems().map(|problem| {
                self.config
                    .user_interface
                    .candidate_details
                    .candidate_problem_style
                    .apply(Cow::Owned(problem.to_string()))
            });

            let similarity = std::iter::once(similarity_percentage)
                .chain(disambiguation_parts)
                .chain(problems)
                .join(
                    &self
                        .config
                        .user_interface
                        .candidate_details
                        .candidate_similarity_separator_style
                        .apply(", ")
                        .to_string(),
                );

            write!(
                f,
                "{release_artist_and_title}{similarity_prefix}{similarity}{similarity_suffix}",
                similarity_prefix = self
                    .config
                    .user_interface
                    .candidate_details
                    .candidate_similarity_prefix_style
                    .apply(
                        &self
                            .config
                            .user_interface
                            .candidate_details
                            .candidate_similarity_prefix
                    ),
                similarity_suffix = self
                    .config
                    .user_interface
                    .candidate_details
                    .candidate_similarity_suffix_style
                    .apply(
                        &self
                            .config
                            .user_interface
                            .candidate_details
                            .candidate_similarity_suffix
                    ),
            )
        } else {
            let text = match &self.option {
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
                self.config
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
    fn into_styled(
        self,
        config: &'a Config,
        disambiguation_needs: DisambiguationNeeds,
    ) -> StyledReleaseCandidateSelectionOption<'a, T> {
        StyledReleaseCandidateSelectionOption {
            config,
            option: self,
            disambiguation_needs,
        }
    }
}

impl<'a, T: ReleaseLike> From<StyledReleaseCandidateSelectionOption<'a, T>>
    for ReleaseCandidateSelectionOption<'a, T>
{
    fn from(value: StyledReleaseCandidateSelectionOption<'a, T>) -> Self {
        value.option
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

    // Compute which fields need disambiguation
    let disambiguation_needs = compute_disambiguation_needs(candidates);

    let options: Vec<StyledReleaseCandidateSelectionOption<'a, T>> = candidates
        .iter()
        .zip(disambiguation_needs.iter())
        .map(|(candidate, &needs)| {
            ReleaseCandidateSelectionOption::Candidate(candidate).into_styled(config, needs)
        })
        .chain(
            additional_options
                .into_iter()
                .map(|option| option.into_styled(config, DisambiguationNeeds::empty())),
        )
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
