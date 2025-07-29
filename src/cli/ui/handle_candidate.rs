// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Show candidate details and select next action.

use super::util::{self, LayoutItem, StyledContentList};
use crate::config::{CandidateDetails, Config, UnmatchedTrackStyleConfig};
use crate::distance::{Distance, UnmatchedTracksSource};
use crate::media::MediaLike;
use crate::pathformat::PathFormatterValues;
use crate::release::ReleaseLike;
use crate::release_candidate::ReleaseCandidate;
use crate::track::{AnalyzedTrackMetadata, InvolvedPerson, TrackLike};
use crate::util::{FormattedDuration, KeyedBinaryHeap};
use crossterm::{
    style::{ContentStyle, Stylize},
    terminal,
};
use inquire::{InquireError, Select};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::iter::Peekable;

/// The result of a `handle_candidate` all.
pub enum HandleCandidateResult {
    /// Apply the current candidate.
    Apply,
    /// Show more details about the current candidate.
    ShowDetails,
    /// Hide details about the current candidate.
    HideDetails,
    /// Print the tracklist (for copy & paste into MusicBrainz).
    PrintTrackList,
    /// Skip the release.
    Skip,
    /// Back to candidate selection.
    BackToSelection,
    /// Stop candidate selection and quit.
    Quit,
}

/// A styled version of `HandleCandidateResult` that is displayed to the user.
struct StyledHandleCandidateResult<'a>(&'a Config, HandleCandidateResult);

impl fmt::Display for StyledHandleCandidateResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match &self.1 {
            HandleCandidateResult::Apply => "Apply candidate",
            HandleCandidateResult::ShowDetails => "Show details",
            HandleCandidateResult::HideDetails => "Hide details",
            HandleCandidateResult::PrintTrackList => "Print Tracklist",
            HandleCandidateResult::Skip => "Skip album",
            HandleCandidateResult::BackToSelection => "Back to candidate selection",
            HandleCandidateResult::Quit => "Quit",
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

/// Print additional metadata.
fn print_extra_metadata(
    lhs: Option<Cow<'_, str>>,
    rhs: Option<Cow<'_, str>>,
    missing_str: &str,
    suffix: &str,
    candidate_details_config: &CandidateDetails,
    max_width: usize,
    max_height: usize,
) {
    let (lhs_value, rhs_value) = util::string_diff_opt(
        lhs,
        rhs,
        missing_str,
        &candidate_details_config.string_diff_style,
    );
    let lhs = LayoutItem::new(lhs_value);
    let rhs = LayoutItem::new(rhs_value).with_suffix(
        candidate_details_config
            .changed_value_style
            .apply(suffix.as_ref())
            .into(),
    );
    util::print_column_layout(
        lhs,
        rhs,
        &candidate_details_config.tracklist_extra_indent,
        &candidate_details_config.tracklist_extra_separator,
        max_width,
        max_height,
    );
}

/// Display details about the candidate.
pub fn show_candidate<B: ReleaseLike, C: ReleaseLike>(
    config: &Config,
    base_release: &B,
    candidate: &ReleaseCandidate<C>,
    show_details: bool,
) {
    let candidate_details_config = &config.user_interface.candidate_details;

    let distance_color = util::distance_color(&candidate.distance(config));

    let release = candidate.release();
    let release_artist_and_title = util::format_release_artist_and_title(release);

    println!(
        "{release_artist_and_title}",
        release_artist_and_title =
            ContentStyle::from(&candidate_details_config.release_artist_and_title_style)
                .with(distance_color)
                .apply(release_artist_and_title),
    );
    println!(
        "Similarity: {similarity}",
        similarity = util::format_similarity(&candidate.distance(config))
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
    println!(
        "{}",
        candidate_details_config
            .release_meta_style
            .apply(release_meta)
    );

    if let Some(mb_url) = release.musicbrainz_release_url() {
        println!(
            "{}",
            candidate_details_config.release_meta_style.apply(mb_url)
        );
    }

    // Show the tracklist of matched and unmatched tracks.
    //
    // First, show the matched tracks.
    let track_assignment = candidate.similarity().track_assignment();
    let matched_track_map = track_assignment.map_rhs_indices_to_lhs();
    let mut rhs_track_index: usize = 0;
    for (media_index, media) in release.media().enumerate() {
        let format = media.media_format().unwrap_or_else(|| "Medium".into());
        let disc_title = if let Some(title) = media.media_title() {
            format!("{format} {index}: {title}", index = media_index + 1)
        } else {
            format!("{format} {index}", index = media_index + 1)
        };

        println!(
            "{}",
            candidate_details_config.disc_title_style.apply(disc_title)
        );

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
                (!track_similarity.track_number.is_equal()).then_some("#"),
                (!track_similarity.track_title.is_equal()).then_some("title"),
                (!track_similarity.track_length.is_equal()).then_some("length"),
                (!show_details
                    && track_similarity.musicbrainz_recording_id.is_present_left()
                    && !track_similarity.musicbrainz_recording_id.is_equal())
                .then_some("id"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
            let rhs_suffix = if changes.is_empty() {
                StyledContentList::default()
            } else {
                StyledContentList::from(
                    candidate_details_config
                        .changed_value_style
                        .apply(Cow::from(format!(" ({changes})"))),
                )
            };

            // Format track title difference.
            let (lhs_track_title, rhs_track_title) = util::string_diff_opt(
                lhs_track
                    .track_title()
                    .or_else(|| lhs_track.track_file_stem()),
                rhs_track
                    .track_title()
                    .or_else(|| rhs_track.track_file_stem()),
                "<unknown title>",
                &candidate_details_config.string_diff_style,
            );

            // Format track number.
            let lhs_track_number = lhs_track.track_number().map_or_else(
                || {
                    candidate_details_config
                        .track_number_style_default
                        .apply(Cow::from(format!("#{lhs_track_index}")))
                },
                |number| candidate_details_config.track_number_style.apply(number),
            );
            let rhs_track_number = rhs_track.track_number().map_or_else(
                || {
                    candidate_details_config
                        .track_number_style_default
                        .apply(Cow::from(format!("#{rhs_track_index}")))
                },
                |number| candidate_details_config.track_number_style.apply(number),
            );

            // Build the main layout item for the tracks.
            let mut lhs =
                LayoutItem::new(lhs_track_title).with_prefix(StyledContentList::new(vec![
                    lhs_track_number,
                    candidate_details_config
                        .track_number_style_default
                        .apply(Cow::from(". ")),
                ]));

            let mut rhs = LayoutItem::new(rhs_track_title)
                .with_prefix(StyledContentList::new(vec![
                    rhs_track_number,
                    candidate_details_config
                        .track_number_style_default
                        .apply(Cow::from(". ")),
                ]))
                .with_suffix(rhs_suffix);

            // Add the track length to the layout item (if different).
            if !track_similarity.track_length.is_equal() {
                lhs.content.push(lhs_track.track_length().map_or_else(
                    || {
                        candidate_details_config
                            .track_length_missing_style
                            .apply(Cow::from(" (?:??)"))
                    },
                    |length| {
                        candidate_details_config
                            .track_length_changed_style
                            .apply(Cow::from(format!(" ({})", length.formatted_duration())))
                    },
                ));

                rhs.content.push(rhs_track.track_length().map_or_else(
                    || {
                        candidate_details_config
                            .track_length_missing_style
                            .apply(Cow::from(" (?:??)"))
                    },
                    |length| {
                        candidate_details_config
                            .track_length_changed_style
                            .apply(Cow::from(format!(" ({})", length.formatted_duration())))
                    },
                ));
            }

            // Finally, print the track title/number/length layout item.
            util::print_column_layout(
                lhs,
                rhs,
                &candidate_details_config.tracklist_indent,
                &candidate_details_config.tracklist_separator,
                max_width,
                candidate_details_config.tracklist_title_line_limit,
            );

            // Print the track artist (if different)
            if !track_similarity.track_artist.is_equal() {
                print_extra_metadata(
                    lhs_track.track_artist(),
                    rhs_track.track_artist(),
                    "<unknown artist>",
                    " (artist)",
                    candidate_details_config,
                    max_width,
                    candidate_details_config.tracklist_artist_line_limit,
                );
            }

            if show_details {
                // TODO: Add more metadata here.
                let lhs_performers = lhs_track.performers();
                let rhs_performers = rhs_track.performers();
                if lhs_performers != rhs_performers {
                    for (involvement, lhs_involvee, rhs_involvee) in match_involved_persons(
                        Box::new(lhs_performers.into_iter().flatten()),
                        Box::new(rhs_performers.into_iter().flatten()),
                    ) {
                        if Distance::between_options_or_minmax(
                            lhs_involvee.as_deref(),
                            rhs_involvee.as_deref(),
                        )
                        .is_equality()
                        {
                            continue;
                        }

                        let unknown_string = format!("<unknown performer[{involvement}]>");
                        print_extra_metadata(
                            lhs_involvee,
                            rhs_involvee,
                            unknown_string.as_str(),
                            " (performer)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }

                // Print the MusicBrain Recording ID (if different)
                if !track_similarity.musicbrainz_recording_id.is_equal() {
                    print_extra_metadata(
                        lhs_track.musicbrainz_recording_id(),
                        rhs_track.musicbrainz_recording_id(),
                        "<unknown id>",
                        " (id)",
                        candidate_details_config,
                        max_width,
                        candidate_details_config.tracklist_extra_line_limit,
                    );
                }

                // Print the AcoustID Fingerprint (if available/different)
                if let Some(fingerprint) = lhs_track.analyzed_metadata().acoustid_fingerprint() {
                    if lhs_track
                        .acoustid_fingerprint()
                        .is_none_or(|f| f != fingerprint)
                    {
                        print_extra_metadata(
                            lhs_track.acoustid_fingerprint(),
                            Some(fingerprint),
                            "<unknown fingerprint>",
                            " (fprint)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }

                // Print the ReplayGain 2.0 Track Gain (if available/different)
                if let Some(gain) = lhs_track.analyzed_metadata().replay_gain_track_gain() {
                    if lhs_track.replay_gain_track_gain().is_none_or(|g| g != gain) {
                        print_extra_metadata(
                            lhs_track.replay_gain_track_gain(),
                            Some(gain),
                            "<unknown gain>",
                            " (gain)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }

                // Print the ReplayGain 2.0 Track Peak (if available/different)
                if let Some(peak) = lhs_track.analyzed_metadata().replay_gain_track_peak() {
                    if lhs_track.replay_gain_track_peak().is_none_or(|p| p != peak) {
                        print_extra_metadata(
                            lhs_track.replay_gain_track_peak(),
                            Some(peak),
                            "<unknown peak>",
                            " (peak)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }

                // Print the ReplayGain 2.0 Track Range (if available/different)
                if let Some(range) = lhs_track.analyzed_metadata().replay_gain_track_range() {
                    if lhs_track
                        .replay_gain_track_range()
                        .is_none_or(|l| l != range)
                    {
                        print_extra_metadata(
                            lhs_track.replay_gain_track_range(),
                            Some(range),
                            "<unknown range>",
                            " (range)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }

                // Print the ReplayGain 2.0 Album Gain (if available/different)
                if let Some(gain) = base_release.replay_gain_album_gain_analyzed() {
                    if lhs_track.replay_gain_album_gain().is_none_or(|g| g != gain) {
                        print_extra_metadata(
                            lhs_track.replay_gain_track_gain(),
                            Some(gain),
                            "<unknown gain>",
                            " (album gain)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }

                // Print the ReplayGain 2.0 Album Peak (if available/different)
                if let Some(peak) = base_release.replay_gain_album_peak_analyzed() {
                    if lhs_track.replay_gain_album_peak().is_none_or(|p| p != peak) {
                        print_extra_metadata(
                            lhs_track.replay_gain_track_peak(),
                            Some(peak),
                            "<unknown peak>",
                            " (album peak)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }

                // Print the ReplayGain 2.0 Album Range (if available/different)
                if let Some(range) = base_release.replay_gain_album_range_analyzed() {
                    if lhs_track
                        .replay_gain_album_range()
                        .is_none_or(|l| l != range)
                    {
                        print_extra_metadata(
                            lhs_track.replay_gain_track_range(),
                            Some(range),
                            "<unknown range>",
                            " (album range)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }
                if let Some(path) = lhs_track.track_path() {
                    let old_path = path;

                    let values = PathFormatterValues::default()
                        .with_release(base_release)
                        .with_release(release)
                        .with_media(media)
                        .with_track(*lhs_track_index + 1, *lhs_track)
                        .with_track(rhs_track_index + 1, rhs_track);
                    let new_path = config
                        .paths
                        .format_path(&values, lhs_track.track_file_extension())
                        .ok();
                    if new_path.as_deref().is_none_or(|p| p != old_path) {
                        print_extra_metadata(
                            old_path.to_str().map(Cow::from),
                            new_path.as_deref().and_then(|x| x.to_str()).map(Cow::from),
                            "<unknown path>",
                            " (path)",
                            candidate_details_config,
                            max_width,
                            candidate_details_config.tracklist_extra_line_limit,
                        );
                    }
                }
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
                println!(
                    "{}",
                    candidate_details_config
                        .unmatched_tracks_residual
                        .headline_style
                        .apply(title)
                );
                print_unmatched_tracks(
                    base_release,
                    &unmatched_track_indices,
                    &candidate_details_config.unmatched_tracks_residual,
                );
            }
            UnmatchedTracksSource::Right => {
                let title = format!(
                    "Missing Tracks ({unmatched_count}/{total_count}):",
                    unmatched_count = unmatched_track_indices.len(),
                    total_count = rhs_track_index
                );
                println!(
                    "{}",
                    candidate_details_config
                        .unmatched_tracks_missing
                        .headline_style
                        .apply(title)
                );
                print_unmatched_tracks(
                    release,
                    &unmatched_track_indices,
                    &candidate_details_config.unmatched_tracks_missing,
                );
            }
        }
    }
}

/// Prompt the user how to handle the candidate.
pub fn handle_candidate<B: ReleaseLike, C: ReleaseLike>(
    config: &Config,
    base_release: &B,
    candidate: &ReleaseCandidate<C>,
) -> Result<HandleCandidateResult, InquireError> {
    let mut show_details = false;
    loop {
        show_candidate(config, base_release, candidate, show_details);
        let options = vec![
            HandleCandidateResult::Apply.into_styled(config),
            if show_details {
                HandleCandidateResult::HideDetails.into_styled(config)
            } else {
                HandleCandidateResult::ShowDetails.into_styled(config)
            },
            HandleCandidateResult::PrintTrackList.into_styled(config),
            HandleCandidateResult::Skip.into_styled(config),
            HandleCandidateResult::BackToSelection.into_styled(config),
            HandleCandidateResult::Quit.into_styled(config),
        ];

        break match Select::new("Select an option:", options).prompt() {
            Ok(StyledHandleCandidateResult(_, HandleCandidateResult::ShowDetails)) => {
                show_details = true;
                continue;
            }
            Ok(StyledHandleCandidateResult(_, HandleCandidateResult::HideDetails)) => {
                show_details = false;
                continue;
            }
            Ok(option) => Ok(option.into()),
            Err(InquireError::OperationCanceled) => Ok(HandleCandidateResult::BackToSelection),
            Err(InquireError::OperationInterrupted) => Ok(HandleCandidateResult::Quit),
            Err(err) => Err(err),
        };
    }
}

/// Print a list of unmatched tracks.
fn print_unmatched_tracks(
    release: &impl ReleaseLike,
    unmatched_track_indices: &HashSet<usize>,
    config: &UnmatchedTrackStyleConfig,
) {
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
            "{prefix}{track_number}{track_number_suffix}{track_title}",
            prefix = config.prefix_style.apply(&config.prefix),
            track_number = config.track_number_style.apply(&track_number),
            track_number_suffix =
                config
                    .track_number_style
                    .apply(if track_number.is_empty() { "" } else { ". " }),
            track_title = config.track_title_style.apply(track_title),
        );
    }
}

/// Iterator struct that is returned by the [`match_involved_persons`] function.
struct InvolvedPersonMatches<'a> {
    /// A sorted, peekable iterator of involved persons for the left-hand side.
    left_iter: Peekable<Box<dyn Iterator<Item = InvolvedPerson<'a>> + 'a>>,
    /// A sorted, peekable iterator of involved persons for the left-hand side.
    right_iter: Peekable<Box<dyn Iterator<Item = InvolvedPerson<'a>> + 'a>>,
}

impl<'a> Iterator for InvolvedPersonMatches<'a> {
    type Item = (Cow<'a, str>, Option<Cow<'a, str>>, Option<Cow<'a, str>>);

    fn next(&mut self) -> Option<Self::Item> {
        let order = match (self.left_iter.peek(), self.right_iter.peek()) {
            (Some(l), Some(r)) => l.involvement.cmp(&r.involvement),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => return None,
        };

        match order {
            Ordering::Less => self
                .left_iter
                .next()
                .map(|item| (item.involvement, Some(item.involvee), None)),
            Ordering::Equal => {
                let left = self.left_iter.next().unwrap();
                let right = self.right_iter.next().unwrap();
                Some((left.involvement, Some(left.involvee), Some(right.involvee)))
            }
            Ordering::Greater => self
                .right_iter
                .next()
                .map(|item| (item.involvement, None, Some(item.involvee))),
        }
    }
}

/// Match involved persons from two unsorted iterators by their involvements.
fn match_involved_persons<'a>(
    lhs_persons: impl Iterator<Item = InvolvedPerson<'a>>,
    rhs_persons: impl Iterator<Item = InvolvedPerson<'a>>,
) -> InvolvedPersonMatches<'a> {
    let mut left_map = BTreeMap::new();
    for person in lhs_persons {
        left_map
            .entry(person.involvement.clone())
            .or_insert(KeyedBinaryHeap::new(|x: &InvolvedPerson<'_>| {
                x.involvee.clone()
            }))
            .push(person);
    }

    let mut right_map = BTreeMap::new();
    for person in rhs_persons {
        right_map
            .entry(person.involvement.clone())
            .or_insert(KeyedBinaryHeap::new(|x: &InvolvedPerson<'a>| {
                x.involvee.clone()
            }))
            .push(person);
    }

    let left_iter: Box<dyn Iterator<Item = InvolvedPerson<'a>> + 'a> = Box::new(
        left_map
            .into_iter()
            .flat_map(|(_left_key, heap)| heap.into_iter()),
    );
    let right_iter: Box<dyn Iterator<Item = InvolvedPerson<'a>> + 'a> = Box::new(
        right_map
            .into_iter()
            .flat_map(|(_right_key, heap)| heap.into_iter()),
    );
    InvolvedPersonMatches {
        left_iter: left_iter.peekable(),
        right_iter: right_iter.peekable(),
    }
}
