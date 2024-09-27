// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Module for the `config` CLI subcommand.

use crate::cache::Cache;
use crate::Config;
use clap::Parser;
use musicbrainz_rs_nova::entity::{
    release::Release as MusicBrainzRelease, release_group::ReleaseGroup as MusicBrainzReleaseGroup,
    search::SearchResult as MusicBrainzSearchResult,
};

/// Command line arguments for the `config` CLI command.
#[derive(Parser, Debug)]
pub struct Args;

/// Run the `cache` command.
#[expect(clippy::needless_pass_by_value)]
pub fn run(_config: &Config, cache: Option<&Cache>, _args: Args) -> crate::Result<()> {
    let Some(cache) = cache else {
        return Err(crate::Error::CacheNotAvailable);
    };

    let (count, size) = cache.get_stats::<_, MusicBrainzRelease>()?;
    println!("Releases: {count} ({size:?} bytes)");

    let (count, size) = cache.get_stats::<_, MusicBrainzReleaseGroup>()?;
    println!("Release Groups: {count} ({size:?} bytes)");

    let (count, size) = cache.get_stats::<_, MusicBrainzSearchResult<MusicBrainzRelease>>()?;
    println!("Release Search Results: {count} ({size:?} bytes)");

    Ok(())
}
