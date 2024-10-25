// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utility functions

use chrono::TimeDelta;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// An iterator that recursively walks through a directory structure and yields a tuple `(path,
/// dirs, files)` for each directory it visits.
///
/// This struct is created by [`walk_dir`]. See its documentation for more.
pub struct DirWalk {
    /// Queued paths that will be visited next.
    queue: VecDeque<PathBuf>,
}

/// Creates an iterator that walks through a directory structure recursively and yields a tuple
/// consisting of the path of current directory and the files and directories in that directory.
pub fn walk_dir(path: PathBuf) -> DirWalk {
    let mut queue = VecDeque::new();
    queue.push_back(path);
    DirWalk { queue }
}

/// Copy the file
pub fn copy_file<S: AsRef<Path>, D: AsRef<Path>>(source: S, destination: D) -> io::Result<()> {
    let dest_filename = destination
        .as_ref()
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(io::Error::other("cannot determine destination file name"))?;
    let dest_dir = destination
        .as_ref()
        .parent()
        .ok_or(io::Error::other("cannot determine destination directory"))?;
    fs::create_dir_all(dest_dir)?;
    let mut temp_destination_file = tempfile::Builder::new()
        .prefix(format!(".helicon.{dest_filename}").as_str())
        .suffix(".tmp")
        .tempfile_in(dest_dir)?;
    let mut source_file = fs::File::open(source)?;
    let _ = io::copy(&mut source_file, &mut temp_destination_file)?;

    // When copying succeeded, persist the temporary file at the actual destination.
    let temp_destination = temp_destination_file.into_temp_path();
    temp_destination.persist(destination)?;

    Ok(())
}

/// Move the file.
pub fn move_file<S: AsRef<Path>, D: AsRef<Path>>(source: S, destination: D) -> crate::Result<()> {
    // First, try renaming.
    if let Ok(()) = fs::rename(&source, &destination) {
        return Ok(());
    }

    // If that didn't work, try to copy the source file to a temporary file on the destination
    // filesystem and persist the temporary file under the actual destination path if this
    // succeeds.
    copy_file(&source, destination)?;

    // Then remove the source file.
    fs::remove_file(source)?;

    Ok(())
}

impl Iterator for DirWalk {
    type Item = io::Result<(PathBuf, Vec<PathBuf>, Vec<PathBuf>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let queued_path = self.queue.pop_front();

        queued_path.map(move |path| {
            fs::read_dir(&path).and_then(move |entries| {
                let mut files = vec![];
                let mut dirs = vec![];
                for entry in entries {
                    let entry_path = entry?.path();

                    if entry_path.is_dir() {
                        dirs.push(entry_path.clone());
                    } else {
                        files.push(entry_path);
                    }
                }

                dirs.sort_unstable();
                files.sort_unstable();

                self.queue.extend(dirs.clone());
                Ok((path, dirs.clone(), files))
            })
        })
    }
}

/// Indicates that a value can be represent a duration as a formatted string.
pub trait FormattedDuration {
    /// Format the duration as a string, either in the form `M:SS` or `H:MM:SS`.
    fn formatted_duration(&self) -> String;
}

impl FormattedDuration for TimeDelta {
    fn formatted_duration(&self) -> String {
        let hours = self.num_hours();
        let minutes = self.num_minutes() - hours * 60;
        let seconds = self.num_seconds() - hours * 60 * 60 - minutes * 60;
        if hours > 0 {
            format!("{hours}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes}:{seconds:02}")
        }
    }
}
