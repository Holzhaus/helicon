// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Main module

use clap::Parser;
use env_logger::{Builder, WriteStyle};
use log::LevelFilter;
use mbtagger::import;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to import.
    path: PathBuf,
    /// Show debug information.
    #[arg(short, long)]
    verbose: bool,
}

impl Args {
    fn log_level_filter(&self) -> LevelFilter {
        if self.verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        }
    }
}

#[tokio::main]
async fn main() -> mbtagger::Result<()> {
    let args = Args::parse();
    Builder::new()
        .filter(None, args.log_level_filter())
        .write_style(WriteStyle::Auto)
        .init();
    import::run(args.path).await
}
