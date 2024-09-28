// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Show candidate details and select next action.

use super::util::{self, LayoutItem, StyledContentList};
use crate::distance::{TrackSimilarity, UnmatchedTracksSource};
use crate::media::MediaLike;
use crate::release::ReleaseLike;
use crate::release_candidate::ReleaseCandidate;
use crate::track::TrackLike;
use crate::Config;
use crossterm::{
    style::{ContentStyle, StyledContent, Stylize},
    terminal,
};
use inquire::{InquireError, Select};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
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

/// A styled version of `HandleCandidateResult` that is displayed to the user.
struct StyledHandleCandidateResult<'a>(&'a Config, HandleCandidateResult);

impl fmt::Display for StyledHandleCandidateResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match &self.1 {
            HandleCandidateResult::Apply => "Apply candidate",
            HandleCandidateResult::Skip => "Skip album",
            HandleCandidateResult::BackToSelection => "Back to candidate selection",
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

impl<'a> HandleCandidateResult {
    /// Style this `HandleCandidateResult` using the styles defined in the `Config`.
    fn into_styled(self, config: &'a Config) -> StyledHandleCandidateResult<'a> {
        StyledHandleCandidateResult(config, self)
    }
}

impl From<StyledHandleCandidateResult<'_>> for HandleCandidateResult {
    fn from(value: StyledHandleCandidateResult<'_>) -> Self {
        value.1
    }
}

/// Display details about the candidate.
pub fn handle_candidate<B: ReleaseLike, C: ReleaseLike>(
    config: &Config,
    base_release: &B,
    candidate: &ReleaseCandidate<C>,
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

    // Calculate maximum width of the terminal.
    let max_width = terminal::size().map_or(
        config.user_interface.default_terminal_width,
        |(cols, _rows)| usize::from(cols),
    );
    let max_width = config
        .user_interface
        .max_terminal_width
        .map_or(max_width, |max| max_width.min(max));

    // Show release metadata
    let release_meta = [
        release.release_media_format(),
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
            if (text.len() + item.len()) > max_width {
                return text;
            }

            text + item.as_ref()
        } else {
            if (text.len() + item.len() + 3) > max_width {
                return text;
            }

            text + " | " + item.as_ref()
        }
    });
    println!("{}", release_meta.grey());

    if let Some(mb_url) = release.musicbrainz_release_url() {
        println!("{}", mb_url.grey());
    }

    // Show the tracklist of matched and unmatched tracks.
    //
    // First, show the matched tracks.
    let track_assignment = candidate.similarity().track_assignment();
    let matched_track_map = track_assignment
        .matched_tracks()
        .map(|pair| (pair.rhs, (pair.lhs, &pair.similarity)))
        .collect::<HashMap<usize, (usize, &TrackSimilarity)>>();
    let mut rhs_track_index: usize = 0;
    for (media_index, media) in release.media().enumerate() {
        let format = media.media_format().unwrap_or_else(|| "Medium".into());
        let disc_title = if let Some(title) = media.media_title() {
            format!("{format} {index}: {title}", index = media_index + 1)
        } else {
            format!("{format} {index}", index = media_index + 1)
        };
        println!("{}", disc_title.underlined());

        for rhs_track in media.media_tracks() {
            let Some((lhs_track_index, track_similarity)) = matched_track_map.get(&rhs_track_index)
            else {
                rhs_track_index += 1;
                continue;
            };

            let Some(lhs_track) = &base_release.release_tracks().nth(*lhs_track_index) else {
                rhs_track_index += 1;
                continue;
            };

            let changes = [
                (!track_similarity.is_track_title_equal()).then_some("title"),
                (!track_similarity.is_track_number_equal()).then_some("number"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
            let rhs_suffix = if changes.is_empty() {
                StyledContentList::default()
            } else {
                StyledContentList::from(
                    ContentStyle::new()
                        .yellow()
                        .bold()
                        .apply(Cow::from(format!(" ({changes})"))),
                )
            };

            let (lhs_track_title, rhs_track_title) = util::string_diff_opt(
                lhs_track.track_title(),
                rhs_track.track_title(),
                "<unknown title>",
                &config.user_interface.candidate_details.string_diff_style,
            );

            let lhs_track_number = util::convert_styled_content(StyledContent::new(
                ContentStyle::new(),
                lhs_track
                    .track_number()
                    .unwrap_or_else(|| Cow::from(format!("#{lhs_track_index}"))),
            ));

            let lhs = LayoutItem::new(lhs_track_title).with_prefix(StyledContentList::new(vec![
                lhs_track_number,
                util::convert_styled_content(". ".grey()),
            ]));

            let rhs_track_number = util::convert_styled_content(StyledContent::new(
                ContentStyle::new(),
                rhs_track
                    .track_number()
                    .unwrap_or_else(|| Cow::from(format!("#{rhs_track_index}"))),
            ));
            let rhs = LayoutItem::new(rhs_track_title)
                .with_prefix(StyledContentList::new(vec![
                    rhs_track_number,
                    util::convert_styled_content(". ".grey()),
                ]))
                .with_suffix(rhs_suffix);

            util::print_column_layout(lhs, rhs, " * ", " -> ", max_width);

            if !track_similarity.is_track_artist_equal() {
                let (lhs_track_artist, rhs_track_artist) = util::string_diff_opt(
                    lhs_track.track_artist(),
                    rhs_track.track_artist(),
                    "<unknown artist>",
                    &config.user_interface.candidate_details.string_diff_style,
                );
                let lhs = LayoutItem::new(lhs_track_artist);
                let rhs = LayoutItem::new(rhs_track_artist).with_suffix(StyledContentList::from(
                    util::convert_styled_content("(artist)".yellow().bold()),
                ));
                util::print_column_layout(lhs, rhs, "   ", " -> ", max_width);
            }

            if !track_similarity.is_musicbrainz_recording_id_equal() {
                let (lhs_mb_rec_id, rhs_mb_rec_id) = util::string_diff_opt(
                    lhs_track.musicbrainz_recording_id(),
                    rhs_track.musicbrainz_recording_id(),
                    "<unknown id>",
                    &config.user_interface.candidate_details.string_diff_style,
                );
                let lhs = LayoutItem::new(lhs_mb_rec_id);
                let rhs = LayoutItem::new(rhs_mb_rec_id).with_suffix(StyledContentList::from(
                    util::convert_styled_content("(id)".yellow().bold()),
                ));
                util::print_column_layout(lhs, rhs, "   ", " -> ", max_width);
            }

            rhs_track_index += 1;
        }
    }

    // Second, show the unmatched ones.
    let unmatched_track_indices = track_assignment
        .unmatched_tracks()
        .iter()
        .copied()
        .collect::<HashSet<usize>>();
    if !unmatched_track_indices.is_empty() {
        match track_assignment.unmatched_tracks_source() {
            UnmatchedTracksSource::Left => {
                let title = format!(
                    "Residual Tracks ({unmatched_count}/{total_count}):",
                    unmatched_count = unmatched_track_indices.len(),
                    total_count = "??"
                );
                println!("{}", title.yellow().underlined());
                print_unmatched_tracks(base_release, &unmatched_track_indices);
            }
            UnmatchedTracksSource::Right => {
                let title = format!(
                    "Missing Tracks ({unmatched_count}/{total_count}):",
                    unmatched_count = unmatched_track_indices.len(),
                    total_count = rhs_track_index
                );
                println!("{}", title.yellow().underlined());
                print_unmatched_tracks(release, &unmatched_track_indices);
            }
        }
    }

    let options = vec![
        HandleCandidateResult::Apply.into_styled(config),
        HandleCandidateResult::Skip.into_styled(config),
        HandleCandidateResult::BackToSelection.into_styled(config),
    ];

    match Select::new("Select an option:", options).prompt() {
        Ok(option) => Ok(option.into()),
        Err(InquireError::OperationCanceled) => Ok(HandleCandidateResult::BackToSelection),
        Err(err) => Err(err),
    }
}

/// Print a list of unmatched tracks.
fn print_unmatched_tracks(release: &impl ReleaseLike, unmatched_track_indices: &HashSet<usize>) {
    for (i, track) in release
        .release_tracks()
        .enumerate()
        .filter(|(i, _)| unmatched_track_indices.contains(i))
    {
        let track_number = track
            .track_number()
            .unwrap_or_else(|| format!("#{index}", index = i + 1).into());
        let track_title = track.track_title().unwrap_or_else(|| "".into());

        println!(
            "  ! {track_number}{track_number_suffix}{track_title}",
            track_number = track_number.grey(),
            track_number_suffix = if track_number.is_empty() { "" } else { ". " }.grey(),
            track_title = track_title.yellow(),
        );
    }
}
