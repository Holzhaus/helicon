// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use crate::analyzer;
use crate::util::FormattedDuration;
use crate::Cache;
use crate::Config;
use base64::prelude::*;
use clap::Parser;
use std::path::PathBuf;

/// Command line arguments for the `import` CLI command.
#[derive(Parser, Debug)]
pub struct Args {
    /// Path of audio file to analyze.
    path: PathBuf,
}

/// Analyze a file.
///
/// # Errors
///
/// If the underlying [`walk_dir`] function encounters any form of I/O or other error, an error
/// variant will be returned.
pub fn run(config: &Config, _cache: Option<&Cache>, args: Args) -> crate::Result<()> {
    let path = args.path;
    let result = analyzer::analyze(config, &path)?;

    if let Some(Ok(track_length)) = result.track_length {
        print!("Track Length: {}", track_length.formatted_duration());
    }

    if let Some(Ok((duration, fingerprint))) = result.chromaprint_fingerprint {
        println!("Duration: {duration}");
        println!(
            "Fingerprint: {}",
            BASE64_URL_SAFE_NO_PAD.encode(fingerprint)
        );
    }

    Ok(())
}
