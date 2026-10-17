// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Tests for the `cache` CLI command.

use std::path::Path;
use std::process::{Command, Output};

/// Run `helicon cache <args>` with the given directory as cache home.
fn run_cache_command(cache_home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_helicon"))
        .arg("cache")
        .args(args)
        .env("XDG_CACHE_HOME", cache_home)
        .output()
        .unwrap()
}

#[test]
fn test_cache_clear_removes_only_cache_items() {
    let dir = tempfile::tempdir().unwrap();
    let release_dir = dir.path().join("helicon/musicbrainz/release");
    let group_dir = dir.path().join("helicon/musicbrainz/release-group");
    std::fs::create_dir_all(&release_dir).unwrap();
    std::fs::create_dir_all(&group_dir).unwrap();
    std::fs::write(release_dir.join("a.json"), "12345").unwrap();
    std::fs::write(group_dir.join("b.json"), "123").unwrap();
    let unrelated = dir.path().join("helicon/other.txt");
    std::fs::write(&unrelated, "keep").unwrap();

    let output = run_cache_command(dir.path(), &["clear"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Removed 2 cached items (8 B)"
    );
    assert!(!release_dir.join("a.json").exists());
    assert!(!group_dir.join("b.json").exists());
    assert!(unrelated.exists());
}

#[test]
fn test_cache_without_subcommand_shows_usage() {
    let dir = tempfile::tempdir().unwrap();
    let release_dir = dir.path().join("helicon/musicbrainz/release");
    std::fs::create_dir_all(&release_dir).unwrap();
    std::fs::write(release_dir.join("a.json"), vec![0_u8; 2048]).unwrap();

    let output = run_cache_command(dir.path(), &[]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Releases: 1 (2 KiB)"), "{stdout}");
    assert_eq!(
        stdout,
        String::from_utf8_lossy(&run_cache_command(dir.path(), &["show"]).stdout)
    );
}
