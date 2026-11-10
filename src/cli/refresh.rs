// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to refreshing the metadata of already tagged files.

use super::import::{import_path, ImportOptions, PromptMode};
use crate::musicbrainz::MusicBrainzClient;
use crate::taggedfilecollection::TransferMode;
use crate::Config;
use clap::Parser;
use std::path::PathBuf;

/// Command line arguments for the `refresh` CLI command.
#[derive(Parser, Debug)]
#[expect(clippy::struct_excessive_bools)]
pub struct Args {
    /// Path to refresh.
    path: PathBuf,

    /// Move tracks into the library, so that their paths match the new metadata.
    #[arg(short = 'm', long = "move", id = "move", overrides_with = "nomove")]
    r#move: bool,

    /// Leave tracks where they are (default, opposite of -m).
    #[arg(short = 'M', long, overrides_with = "move")]
    nomove: bool,

    /// Write new metadata to files' tags (default).
    #[arg(short = 'w', long, overrides_with = "nowrite")]
    write: bool,

    /// Don't write metadata (opposite of -w).
    #[arg(short = 'W', long, overrides_with = "write")]
    nowrite: bool,
}

impl Args {
    /// Determine how files should be transferred into the library.
    fn transfer_mode(&self) -> TransferMode {
        if self.r#move {
            TransferMode::Move
        } else {
            TransferMode::InPlace
        }
    }
}

/// Refresh the metadata of all collections below the given path that are tagged with a
/// MusicBrainz release ID.
///
/// Collections without a consensual release ID are skipped. The user is never prompted for input.
///
/// # Errors
///
/// If the underlying [`walk_dir`](crate::util::walk_dir) function encounters any form of I/O or
/// other error, an error variant will be returned.
pub async fn run(
    config: &Config,
    musicbrainz: &MusicBrainzClient,
    args: Args,
) -> crate::Result<()> {
    let options = ImportOptions {
        transfer_mode: args.transfer_mode(),
        write_tags: !args.nowrite,
        prompt_mode: PromptMode::Never,
        refresh_only: true,
    };
    import_path(config, musicbrainz, args.path, options).await
}
