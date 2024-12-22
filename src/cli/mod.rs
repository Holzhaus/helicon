// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Command line interface.

mod analyze;
mod cache;
mod config;
mod import;
mod ui;

use crate::{Cache, Config, USER_AGENT};
use clap::{Parser, Subcommand};
use log::LevelFilter;
use simplelog::{ConfigBuilder as LogConfigBuilder, WriteLogger};
use std::borrow::Cow;
use std::fs::OpenOptions;
use std::path::PathBuf;
use xdg::BaseDirectories;

/// Command line Arguments.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Command to run
    #[command(subcommand)]
    command: Commands,
    /// Show debug information.
    #[arg(short, long)]
    verbose: bool,
    /// Path to configuration file.
    #[arg(short, long, required = false)]
    config_path: Option<PathBuf>,
}

/// Supported CLI Commands.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Show the current cache usage.
    Cache(cache::Args),
    /// Show your current configuration.
    Config(config::Args),
    /// Import files into your collection.
    Import(import::Args),
    /// Analyze a file.
    Analyze(analyze::Args),
}

impl Args {
    /// Get the desired log level, depending on the verbose flag passed on the command line.
    fn log_level_filter(&self) -> LevelFilter {
        if self.verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        }
    }
}

/// Main entry point.
///
/// # Errors
///
/// Can returns errors if the command line arguments are incorrect or the executed programs lead to
/// an error.
///
/// # Panics
///
/// May panic if logging cannot be initialized.
pub async fn main() -> crate::Result<()> {
    let args = Args::parse();

    let base_dirs = BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))?;

    // Initialize logging
    let logfile_path = base_dirs.place_state_file("helicon.log")?;
    let logfile = OpenOptions::new().append(true).open(logfile_path)?;
    WriteLogger::init(
        args.log_level_filter(),
        LogConfigBuilder::new()
            .add_filter_ignore_str("symphonia_core::probe")
            .build(),
        logfile,
    )
    .expect("Failed to initialize logging");

    // Load configuration
    let config = base_dirs
        .find_config_files("config.toml")
        .map(Cow::from)
        .chain(args.config_path.iter().map(Cow::from))
        .fold(Config::builder().with_defaults(), |builder, path| {
            builder.with_file(path)
        })
        .build()?;

    // Initialize cache
    let cache = Cache::new(base_dirs);

    // Set User-Agent header for MusicBrainz requests. This is mandatory to comply with
    // MusicBrainz's API application identification rules.
    //
    // See this for details:
    // - <https://musicbrainz.org/doc/MusicBrainz_API#Application_rate_limiting_and_identification>
    musicbrainz_rs_nova::config::set_user_agent(USER_AGENT);

    match args.command {
        Commands::Import(cmd_args) => import::run(&config, Some(&cache), cmd_args).await,
        Commands::Config(cmd_args) => config::run(&config, Some(&cache), cmd_args),
        Commands::Cache(cmd_args) => cache::run(&config, Some(&cache), cmd_args),
        Commands::Analyze(cmd_args) => analyze::run(&config, Some(&cache), cmd_args),
    }
}
