// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Time-related utility functions.

use chrono::TimeDelta;

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
