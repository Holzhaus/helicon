// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utility functions for formatting sizes.

/// Format a byte count in a short human-readable form (e.g. `32 KiB`).
///
/// Values of 1 KiB or more are shown with one decimal place, except for whole numbers.
#[must_use]
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut unit = 0;
    let mut tenths = bytes.saturating_mul(10);
    // Work in tenths of the unit to avoid floating point arithmetic.
    while tenths >= 10 * 1024 && unit + 1 < UNITS.len() {
        tenths = tenths.div_ceil(1024);
        unit += 1;
    }
    let (whole, fraction) = (tenths / 10, tenths % 10);
    if fraction == 0 {
        format!("{whole} {}", UNITS[unit])
    } else {
        format!("{whole}.{fraction} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1 KiB");
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size(32 * 1024), "32 KiB");
        assert_eq!(format_size(5 * 1024 * 1024 + 512 * 1024), "5.5 MiB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3 GiB");
    }
}
