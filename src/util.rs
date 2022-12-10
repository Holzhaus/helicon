// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utility functions

use std::collections::VecDeque;
use std::fs::read_dir;
use std::io;
use std::path::PathBuf;

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

impl Iterator for DirWalk {
    type Item = io::Result<(PathBuf, Vec<PathBuf>, Vec<PathBuf>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let queued_path = self.queue.pop_front();

        queued_path.map(move |path| {
            read_dir(&path).and_then(move |entries| {
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

                self.queue.extend(dirs.clone().into_iter());
                Ok((path, dirs.clone(), files))
            })
        })
    }
}
