// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Module for the `config` CLI subcommand.

use crate::{Cache, Config};
use clap::Parser;

/// Command line arguments for the `config` CLI command.
#[derive(Parser, Debug)]
pub struct Args;

/// Run the `config` command.
#[expect(clippy::unnecessary_wraps)]
pub fn run(config: &Config, _cache: Option<&Cache>, _args: Args) -> crate::Result<()> {
    let toml_string = toml::to_string_pretty(&config).expect("Failed to serialize configuration");
    println!("{toml_string}");

    Ok(())
}
