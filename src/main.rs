// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
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

#[tokio::main]
async fn main() -> mbtagger::Result<()> {
    let args = Args::parse();
    let log_level = if args.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    Builder::new()
        .filter(None, log_level)
        .write_style(WriteStyle::Auto)
        .init();
    import::run(args.path).await
}
