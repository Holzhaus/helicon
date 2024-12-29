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

use crate::{Cache, Config, PKG_NAME, PKG_VERSION, USER_AGENT};
use clap::{Parser, Subcommand};
use log::LevelFilter;
use simplelog::{ConfigBuilder as LogConfigBuilder, WriteLogger};
use std::borrow::Cow;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use xdg::BaseDirectories;

/// Command line Arguments.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Command to run
    #[command(subcommand)]
    command: Commands,
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

/// Append a numeric suffix (e.g., `.1`) to a path.
fn append_numeric_suffix_to_path(base_path: impl AsRef<Path>, number: usize) -> PathBuf {
    let suffix: OsString = format!(".{number}").into();
    let new_extension = base_path.as_ref().extension().map_or_else(
        || OsString::from(&suffix),
        |ext| {
            let mut extension = ext.to_os_string();
            extension.push(&suffix);
            extension
        },
    );
    base_path.as_ref().with_extension(new_extension)
}

/// Rotate logfiles by renaming `<log>` to `<log>.0`, `<log>.1` to `<log>.2`, etc.
fn rotate_logfiles(base_path: impl AsRef<Path>) -> io::Result<()> {
    let paths_to_rename = (0..7)
        .rev()
        .map(|i| {
            (
                append_numeric_suffix_to_path(&base_path, i),
                append_numeric_suffix_to_path(&base_path, i + 1),
            )
        })
        .chain(std::iter::once((
            base_path.as_ref().to_path_buf(),
            append_numeric_suffix_to_path(&base_path, 0),
        )));
    for (old_path, new_path) in paths_to_rename {
        fs::rename(old_path, new_path).or_else(|err| match err.kind() {
            io::ErrorKind::NotFound => Ok(()),
            _ => Err(err),
        })?;
    }

    Ok(())
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

    let base_dirs = BaseDirectories::with_prefix(PKG_NAME)?;

    // Initialize logging
    let logfile_path = base_dirs.place_state_file(format!("{PKG_NAME}.log"))?;
    rotate_logfiles(&logfile_path)?;
    let logfile = File::create(logfile_path)?;
    WriteLogger::init(
        LevelFilter::Debug,
        LogConfigBuilder::new()
            .add_filter_ignore_str("symphonia_core::probe")
            .build(),
        logfile,
    )
    .expect("Failed to initialize logging");
    log::info!("Started {PKG_NAME} {PKG_VERSION}");

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
