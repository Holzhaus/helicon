// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Module for the `cache` CLI subcommand.

use crate::cache::Cache;
use crate::coverart::CoverArt;
use crate::util::format_size;
use crate::Config;
use clap::{Parser, Subcommand};
use musicbrainz_rs::entity::{
    release::Release as MusicBrainzRelease, release_group::ReleaseGroup as MusicBrainzReleaseGroup,
    search::SearchResult as MusicBrainzSearchResult,
};

/// Command line arguments for the `cache` CLI command.
#[derive(Parser, Debug)]
pub struct Args {
    /// What to do with the cache (default: show usage).
    #[command(subcommand)]
    command: Option<Command>,
}

/// Subcommands of the `cache` CLI command.
#[derive(Subcommand, Debug)]
enum Command {
    /// Show the current cache usage.
    Show,
    /// Remove all items from the cache.
    Clear,
}

/// Run the `cache` command.
pub fn run(_config: &Config, cache: Option<&Cache>, args: Args) -> crate::Result<()> {
    let Some(cache) = cache else {
        return Err(crate::Error::CacheNotAvailable);
    };

    match args.command.unwrap_or(Command::Show) {
        Command::Show => show(cache),
        Command::Clear => clear(cache),
    }
}

/// Print the cache usage.
fn show(cache: &Cache) -> crate::Result<()> {
    let (count, size) = cache.get_stats::<MusicBrainzRelease>()?;
    println!("Releases: {count} ({size})", size = format_size(size));

    let (count, size) = cache.get_stats::<MusicBrainzReleaseGroup>()?;
    println!("Release Groups: {count} ({size})", size = format_size(size));

    let (count, size) = cache.get_stats::<MusicBrainzSearchResult<MusicBrainzRelease>>()?;
    println!(
        "Release Search Results: {count} ({size})",
        size = format_size(size)
    );

    let (count, size) = cache.get_stats::<CoverArt>()?;
    println!("Front Covers: {count} ({size})", size = format_size(size));

    Ok(())
}

/// Remove all cached items.
fn clear(cache: &Cache) -> crate::Result<()> {
    let results = [
        cache.clear_items::<MusicBrainzRelease>()?,
        cache.clear_items::<MusicBrainzReleaseGroup>()?,
        cache.clear_items::<MusicBrainzSearchResult<MusicBrainzRelease>>()?,
        cache.clear_items::<CoverArt>()?,
    ];
    let (count, size) = results
        .iter()
        .fold((0, 0), |(count, size), (c, s)| (count + c, size + s));
    println!(
        "Removed {count} cached items ({size})",
        size = format_size(size)
    );

    Ok(())
}
