// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions related to importing files.

use crate::lookup::find_album_info;
use crate::tag::TaggedFile;
use crate::util::walk_dir;
use std::collections::HashSet;
use std::path::PathBuf;

/// Run an import.
///
/// # Errors
///
/// If the underlying [`walk_dir`] function encounters any form of I/O or other error, an error
/// variant will be returned.
pub fn run(input_path: PathBuf) -> crate::Result<()> {
    let supported_extensions = HashSet::from(["mp3", "flac"]);
    for item in walk_dir(input_path) {
        let (path, _dirs, files) = item?;
        let tagged_files: Vec<TaggedFile> = files
            .iter()
            .filter(|path| {
                path.extension()
                    .map(std::ffi::OsStr::to_ascii_lowercase)
                    .and_then(|extension| {
                        extension
                            .to_str()
                            .map(|extension| supported_extensions.contains(extension))
                    })
                    .unwrap_or(false)
            })
            .filter_map(|path| match TaggedFile::read_from_path(path) {
                Ok(file) => Some(file),
                Err(err) => {
                    log::warn!("Failed to read {}: {}", path.display(), err);
                    None
                }
            })
            .collect();
        if tagged_files.is_empty() {
            continue;
        }

        log::info!(
            "Tagging: {} ({} tracks)",
            path.display(),
            tagged_files.len()
        );
        find_album_info(&tagged_files);
    }

    Ok(())
}
