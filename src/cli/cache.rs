// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Module for the `config` CLI subcommand.

use crate::Cache;
use crate::Config;
use clap::Parser;
use std::io;

/// Command line arguments for the `config` CLI command.
#[derive(Parser, Debug)]
pub struct Args;

/// Run the `cache` command.
#[expect(clippy::needless_pass_by_value)]
pub fn run(_config: &Config, cache: Option<&impl Cache>, _args: Args) -> crate::Result<()> {
    let Some(cache) = cache else {
        return Err(crate::Error::CacheNotAvailable);
    };

    let files = cache.cached_releases();
    let file_count = files.len();
    let file_size = files
        .iter()
        .map(|file| file.metadata().map(|metadata| metadata.len()))
        .sum::<io::Result<u64>>()?;
    println!("Releases: {file_count} ({file_size:?} bytes)");

    let files = cache.cached_release_search_results();
    let file_count = files.len();
    let file_size = files
        .iter()
        .map(|file| file.metadata().map(|metadata| metadata.len()))
        .sum::<io::Result<u64>>()?;
    println!("Release Search Results: {file_count} ({file_size:?} bytes)");

    Ok(())
}
