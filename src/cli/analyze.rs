// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use crate::analyzer;
use crate::config::AnalyzerType;
use crate::util::FormattedDuration;
use crate::Cache;
use crate::Config;
use clap::Parser;
use std::path::PathBuf;

/// Command line arguments for the `import` CLI command.
#[derive(Parser, Debug)]
pub struct Args {
    /// Path of audio file to analyze.
    path: PathBuf,
    /// Use all analyzers (regardless of configuration).
    #[arg(short, long)]
    all: bool,
}

/// Analyze a file.
///
/// # Errors
///
/// If the underlying [`walk_dir`] function encounters any form of I/O or other error, an error
/// variant will be returned.
pub fn run(config: &Config, _cache: Option<&Cache>, args: Args) -> crate::Result<()> {
    let path = args.path;
    let result = if args.all {
        let mut config = config.clone();
        config.analyzers.enabled = vec![
            AnalyzerType::TrackLength,
            AnalyzerType::ChromaprintFingerprint,
            AnalyzerType::EbuR128,
        ];
        analyzer::analyze(&config, &path)?
    } else {
        analyzer::analyze(config, &path)?
    };

    if let Some(result) = result.track_length {
        match result {
            Ok(track_length) => {
                println!("Track Length: {}", track_length.formatted_duration());
            }
            Err(err) => eprintln!("Track Length analysis failed: {err}"),
        }
    }

    if let Some(result) = result.chromaprint_fingerprint {
        match result {
            Ok(result) => {
                println!("Duration: {}", result.duration);
                println!("Fingerprint: {}", result.fingerprint_string(),);
            }
            Err(err) => eprintln!("Chromaprint fingerprint analysis failed: {err}"),
        }
    }

    if let Some(result) = result.ebur128 {
        match result {
            Ok(ebur128) => {
                println!("Track Gain: {}", ebur128.replaygain_track_gain());
                println!("Track Peak: {}", ebur128.peak);
            }
            Err(err) => eprintln!("EBU R 128 analysis failed: {err}"),
        }
    }

    Ok(())
}
