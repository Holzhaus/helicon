// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use super::ui;
use crate::coverart::CoverInfo;
use crate::musicbrainz::{MusicBrainzClient, MusicBrainzRelease};
use crate::release::ReleaseLike;
use crate::release_candidate::{ReleaseCandidate, ReleaseCandidateCollection};
use crate::scanner::Scanner;
use crate::taggedfilecollection::TransferMode;
use crate::{Config, TaggedFileCollection};
use clap::{Parser, ValueEnum};
use crossterm::style::Stylize;
use futures::StreamExt;
use std::path::PathBuf;

/// Command line arguments for the `import` CLI command.
#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools)]
pub struct Args {
    /// Path to import.
    path: PathBuf,

    /// Copy tracks into library directory (default).
    #[arg(short = 'c', long, overrides_with = "nocopy")]
    copy: bool,

    /// Don't copy tracks (opposite of -c).
    #[arg(short = 'C', long, overrides_with = "copy")]
    nocopy: bool,

    /// Move tracks into the library (overrides -c).
    #[arg(short = 'm', long = "move", id = "move", overrides_with = "nomove")]
    r#move: bool,

    /// Don't move tracks into the library (overrides -m).
    #[arg(short = 'M', long, overrides_with = "move")]
    nomove: bool,

    /// Write new metadata to files' tags (default).
    #[arg(short = 'w', long, overrides_with = "nowrite")]
    write: bool,

    /// Don't write metadata (opposite of -w).
    #[arg(short = 'W', long, overrides_with = "write")]
    nowrite: bool,

    /// Only consider collections with an existing MusicBrainz release ID and skip all others.
    #[arg(long)]
    refresh: bool,

    /// When to prompt for input.
    ///
    /// `always` confirms all actions (same as --timid). `never` imports only strongly
    /// recommended candidates and skips the rest (same as --quiet).
    #[arg(long, value_enum, conflicts_with_all = ["timid", "quiet"])]
    prompt: Option<PromptMode>,

    /// Always confirm all actions (same as --prompt=always).
    #[arg(short = 't', long, conflicts_with = "quiet")]
    timid: bool,

    /// Never prompt for input; import only strongly recommended candidates and skip the rest
    /// (same as --prompt=never).
    #[arg(short = 'q', long, conflicts_with = "timid")]
    quiet: bool,
}

/// Determines when the user is prompted for input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
enum PromptMode {
    /// Prompt only if there is no strong recommendation.
    #[default]
    Auto,
    /// Always confirm all actions.
    Always,
    /// Never prompt; skip collections without a strong recommendation.
    Never,
}

impl Args {
    /// Determine when the user should be prompted for input.
    fn prompt_mode(&self) -> PromptMode {
        if self.timid {
            PromptMode::Always
        } else if self.quiet {
            PromptMode::Never
        } else {
            self.prompt.unwrap_or_default()
        }
    }

    /// Determine how files should be transferred into the library.
    fn transfer_mode(&self) -> TransferMode {
        if self.r#move {
            TransferMode::Move
        } else if !self.nocopy {
            TransferMode::Copy
        } else {
            TransferMode::InPlace
        }
    }
}

/// Result returned from the [`select_release()`] function.
#[allow(clippy::large_enum_variant)]
enum SelectionResult {
    /// A candidate was selected and should be assigned to the track collection.
    Selected(TaggedFileCollection, ReleaseCandidate<MusicBrainzRelease>),
    /// Skip importing the track collection.
    Skipped,
    /// Quit import.
    Quit,
}

/// Show the details of the automatically selected candidate.
async fn show_autoselected(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    track_collection: &TaggedFileCollection,
    candidate: &ReleaseCandidate<MusicBrainzRelease>,
) {
    let cover_art = build_cover_art_preview(config, musicbrainz, track_collection, candidate).await;
    ui::show_candidate(
        config,
        track_collection,
        candidate,
        cover_art.as_ref(),
        false,
    );
}

/// Select the release for the given track collection from the list of candidates.
async fn select_release(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    track_collection: TaggedFileCollection,
    mut candidates: ReleaseCandidateCollection<MusicBrainzRelease>,
    prompt_mode: PromptMode,
) -> crate::Result<SelectionResult> {
    println!(
        "Tagging: {release} ({track_count} tracks)",
        release = format!(
            "{artist} - {title}",
            artist = track_collection
                .release_artist()
                .unwrap_or("[unknown artist]".into()),
            title = track_collection
                .release_title()
                .unwrap_or("[unknown album]".into()),
        )
        .bold(),
        track_count = track_collection.release_track_count().unwrap_or(0),
    );
    if let Some(directory) = track_collection.common_directory() {
        println!(
            "{}",
            config
                .user_interface
                .candidate_details
                .release_meta_style
                .apply(directory.display())
        );
    }
    if prompt_mode == PromptMode::Never {
        let Some(best_candidate) = ui::autoselectable_candidate(config, &candidates) else {
            println!("Skipping: no strong recommendation");
            return Ok(SelectionResult::Skipped);
        };
        show_autoselected(config, musicbrainz, &track_collection, best_candidate).await;
        let candidate_index = candidates.find_index(best_candidate);
        return Ok(SelectionResult::Selected(
            track_collection,
            candidates.select_index(candidate_index),
        ));
    }
    let mut allow_autoselection = true;
    'select_candidate: loop {
        let autoselected =
            allow_autoselection && ui::autoselectable_candidate(config, &candidates).is_some();
        let selected_candidate: &ReleaseCandidate<_> = loop {
            match ui::select_candidate(config, &candidates, allow_autoselection)? {
                ui::ReleaseCandidateSelectionResult::Candidate(candidate) => break candidate,
                ui::ReleaseCandidateSelectionResult::FetchCandidateRelease(release_id) => {
                    log::debug!("Manually adding release candidate with release ID {release_id}");
                    let release = musicbrainz.find_release_by_id(release_id).await?;
                    candidates.add_release(release, &track_collection, config);
                }
                ui::ReleaseCandidateSelectionResult::FetchCandidateReleaseGroup(
                    release_group_id,
                ) => {
                    log::debug!("Manually adding release candidate with release group ID {release_group_id}");
                    candidates = musicbrainz
                        .find_releases_by_release_group_id(release_group_id)
                        .await?
                        .fold(candidates, |mut acc, result| async {
                            let release = match result {
                                Ok(release) => release,
                                Err(err) => {
                                    log::warn!("Failed to retrieve release: {err}");
                                    return acc;
                                }
                            };

                            acc.add_release(release, &track_collection, config);
                            acc
                        })
                        .await;
                }
                ui::ReleaseCandidateSelectionResult::PrintTrackList => {
                    ui::print_tracklist(&track_collection);
                    continue 'select_candidate;
                }
                #[cfg(feature = "dev")]
                ui::ReleaseCandidateSelectionResult::DumpReleaseInfo => {
                    use crate::util::FakeRelease;
                    use std::fs::File;

                    let tmp_dir = tempfile::Builder::new()
                        .prefix("helicon-release-info-dump-")
                        .tempdir()?;
                    let mut saved_anything = false;
                    'save_debuginfo: for (path, release) in std::iter::once((
                        tmp_dir.path().join("release.json"),
                        FakeRelease::from(&track_collection),
                    ))
                    .chain(candidates.iter().enumerate().map(|(i, candidate)| {
                        (
                            tmp_dir.path().join(format!("candidate_{i}.json")),
                            FakeRelease::from(candidate.release()),
                        )
                    })) {
                        let file = match File::create(path.as_path()) {
                            Ok(f) => f,
                            Err(err) => {
                                eprintln!(
                                    "Failed to open {path} for writing: {err}",
                                    path = path.display()
                                );
                                continue 'save_debuginfo;
                            }
                        };

                        if let Err(err) = serde_json::to_writer_pretty(file, &release) {
                            eprintln!(
                                "Failed to write debug info to {path}: {err}",
                                path = path.display()
                            );
                            continue 'save_debuginfo;
                        }
                        saved_anything = true;
                    }

                    if saved_anything {
                        eprintln!("Saved debug info to {}", tmp_dir.keep().display());
                    } else {
                        eprintln!("Nothing to do");
                    }

                    continue 'select_candidate;
                }
                ui::ReleaseCandidateSelectionResult::Skipped => {
                    return Ok(SelectionResult::Skipped)
                }
                ui::ReleaseCandidateSelectionResult::Quit => return Ok(SelectionResult::Quit),
            };
        };
        allow_autoselection = false;

        if autoselected && prompt_mode != PromptMode::Always {
            show_autoselected(config, musicbrainz, &track_collection, selected_candidate).await;
            let candidate_index = candidates.find_index(selected_candidate);
            let selected_candidate = candidates.select_index(candidate_index);
            return Ok(SelectionResult::Selected(
                track_collection,
                selected_candidate,
            ));
        }

        let cover_art =
            build_cover_art_preview(config, musicbrainz, &track_collection, selected_candidate)
                .await;
        match ui::handle_candidate(
            config,
            &track_collection,
            selected_candidate,
            cover_art.as_ref(),
        )? {
            ui::HandleCandidateResult::Apply => {
                let candidate_index = candidates.find_index(selected_candidate);
                let selected_candidate = candidates.select_index(candidate_index);
                return Ok(SelectionResult::Selected(
                    track_collection,
                    selected_candidate,
                ));
            }
            ui::HandleCandidateResult::Skip => {
                log::warn!("Skipping collection");
                return Ok(SelectionResult::Skipped);
            }
            ui::HandleCandidateResult::BackToSelection => (),
            ui::HandleCandidateResult::Quit => {
                return Ok(SelectionResult::Quit);
            }
            ui::HandleCandidateResult::ShowDetails
            | ui::HandleCandidateResult::HideDetails
            | ui::HandleCandidateResult::PrintTrackList => {
                unreachable!()
            }
        }
    }
}

/// Fetch the front cover of the candidate to preview what will be embedded.
async fn build_cover_art_preview(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    track_collection: &TaggedFileCollection,
    candidate: &ReleaseCandidate<MusicBrainzRelease>,
) -> Option<ui::CoverArtPreview> {
    let cover_art_config = config.cover_art;
    if !cover_art_config.enabled {
        return None;
    }

    let new_cover = match musicbrainz
        .find_front_cover_by_release_id(&candidate.release().id)
        .await
    {
        Ok(cover) => cover.map(|cover| cover.description(cover_art_config.resolution)),
        Err(err) => {
            log::warn!("Failed to fetch front cover art: {err}");
            None
        }
    };
    Some(ui::CoverArtPreview {
        existing_covers: track_collection
            .front_cover_infos()
            .iter()
            .map(|info| info.as_ref().map(CoverInfo::description))
            .collect(),
        new_cover,
        overwrite_existing: cover_art_config.overwrite_existing,
    })
}

/// Run an import.
///
/// # Errors
///
/// If the underlying [`walk_dir`] function encounters any form of I/O or other error, an error
/// variant will be returned.
pub async fn run(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    args: Args,
) -> crate::Result<()> {
    let transfer_mode = args.transfer_mode();
    let write_tags = !args.nowrite;
    let prompt_mode = args.prompt_mode();
    let mut scanner = Scanner::scan(config.clone(), musicbrainz.clone(), args.path, args.refresh);

    let (importer_tx, mut importer_rx) = tokio::sync::mpsc::channel::<(
        TaggedFileCollection,
        ReleaseCandidate<MusicBrainzRelease>,
    )>(20);
    let cloned_config = config.clone();
    let cloned_musicbrainz = musicbrainz.clone();
    let importer_handle = tokio::task::spawn(async move {
        while let Some((track_collection, selected_candidate)) = importer_rx.recv().await {
            let mut track_collection = track_collection.assign_tags(&selected_candidate);
            if let Err(err) = track_collection.transfer_files(&cloned_config, transfer_mode) {
                log::error!("Failed to transfer files: {err}");
                continue;
            }

            let cover_art_config = cloned_config.cover_art;
            if cover_art_config.enabled
                && track_collection.needs_front_cover(cover_art_config.overwrite_existing)
            {
                match cloned_musicbrainz
                    .find_front_cover_by_release_id(&selected_candidate.release().id)
                    .await
                {
                    Ok(Some(cover)) => {
                        log::info!(
                            "Embedding front cover art ({}, {} bytes)",
                            cover.mime_type(),
                            cover.data().len()
                        );
                        track_collection
                            .set_front_cover(&cover, cover_art_config.overwrite_existing);
                    }
                    Ok(None) => log::info!("No front cover art available for this release"),
                    Err(err) => log::warn!("Failed to fetch front cover art: {err}"),
                }
            }

            if write_tags {
                if let Err(err) = track_collection.write_tags() {
                    log::error!("Failed to write tags: {err}");
                }
            }

            #[cfg(unix)]
            if let Err(err) = track_collection.set_permissions(&cloned_config) {
                log::error!("Failed to set permissions: {err}");
            }
        }
    });

    let mut first_collection = true;
    while let Some(result) = scanner.recv().await {
        let (track_collection, candidates) = match result {
            Ok(res) => res,
            Err(err) => {
                log::error!("Scan of {} failed: {}", err.path.display(), err.source);
                continue;
            }
        };
        if !first_collection {
            println!();
        }
        first_collection = false;
        match select_release(
            config,
            musicbrainz,
            track_collection,
            candidates,
            prompt_mode,
        )
        .await?
        {
            SelectionResult::Selected(track_collection, selected_candidate) => {
                if let Err(err) = importer_tx
                    .send((track_collection, selected_candidate))
                    .await
                {
                    log::error!("Failed to send job to importer: {err}");
                };
            }
            SelectionResult::Skipped => (),
            SelectionResult::Quit => {
                break;
            }
        };
    }

    drop(importer_tx);
    importer_handle.await.unwrap();

    Ok(())
}
