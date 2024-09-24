// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Command line interface.

mod cache;
mod config;
mod import;
mod ui;

use crate::Config;
use clap::{Parser, Subcommand};
use env_logger::{Builder, WriteStyle};
use log::LevelFilter;
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

    /// Get the current configuration.
    fn config(&self) -> crate::Result<Config> {
        match &self.config_path {
            Some(path) => Config::load_from_path(path).map(|config| config.with_defaults()),
            None => Ok(Config::default()),
        }
    }
}

/// Main entry point.
///
/// # Errors
///
/// Can returns errors if the command line arguments are incorrect or the executed programs lead to
/// an error.
pub async fn main() -> crate::Result<()> {
    let args = Args::parse();
    let config = args.config()?;
    let cache = BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))?;

    Builder::new()
        .filter(None, args.log_level_filter())
        .write_style(WriteStyle::Auto)
        .init();

    match args.command {
        Commands::Import(cmd_args) => import::run(&config, Some(&cache), cmd_args).await,
        Commands::Config(cmd_args) => config::run(&config, Some(&cache), cmd_args),
        Commands::Cache(cmd_args) => cache::run(&config, Some(&cache), cmd_args),
    }
}
