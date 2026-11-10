// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Tests for the `refresh` CLI command.

use std::path::Path;
use std::process::{Command, Output, Stdio};

/// Run `helicon <args>` with all XDG directories below the given directory.
fn run_command(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_helicon"))
        .args(args)
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[cfg(feature = "flac")]
#[test]
fn test_refresh_skips_release_without_musicbrainz_id() {
    let home = tempfile::tempdir().unwrap();
    let music = tempfile::tempdir().unwrap();
    let album = music.path().join("Album");
    std::fs::create_dir_all(&album).unwrap();
    let track = album.join("01.flac");
    let _ = std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/media/untagged/track.flac"
        ),
        &track,
    )
    .unwrap();
    let original = std::fs::read(&track).unwrap();

    let output = run_command(home.path(), &["refresh", music.path().to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("no consensual MusicBrainz release ID"),
        "{stderr}"
    );
    assert_eq!(std::fs::read(&track).unwrap(), original);
}

#[test]
fn test_import_no_longer_accepts_refresh_flag() {
    let home = tempfile::tempdir().unwrap();
    let output = run_command(home.path(), &["import", "--refresh", "."]);
    assert!(!output.status.success());
}
