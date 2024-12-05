// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Time-related utility functions.

use chrono::{
    format::{parse, Parsed, StrftimeItems},
    NaiveDate, TimeDelta,
};

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

/// Allowed date formats (as specified in a tag field).
const PARTIAL_DATE_FORMATS: [&str; 5] = ["%Y-%m-%d", "%Y-%m", "%Y%m%d", "%Y%m", "%Y"];

/// Parse a date from a [`str`] slice by trying various common formats.
fn parse_partial_date_from_str(value: impl AsRef<str>) -> Option<NaiveDate> {
    for fmt in PARTIAL_DATE_FORMATS {
        let mut parsed = Parsed::new();
        if parse(&mut parsed, value.as_ref(), StrftimeItems::new(fmt)).is_err() {
            continue;
        }

        if let Some(date) = parsed
            .year()
            .map(|year| {
                parsed
                    .month
                    .map_or((year, 1, 1), |month| (year, month, parsed.day.unwrap_or(1)))
            })
            .and_then(|(year, month, day)| NaiveDate::from_ymd_opt(year, month, day))
        {
            return Some(date);
        }
    }

    None
}

/// Parse the year from a [`str`] slice and return a [`String`] if found.
pub fn parse_year_from_str(value: &str) -> Option<String> {
    parse_partial_date_from_str(value).map(|date| date.format("%Y").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_date_from_str() {
        assert_eq!(
            parse_partial_date_from_str("1962-01-17"),
            Some(NaiveDate::from_ymd_opt(1962, 1, 17).unwrap())
        );
        assert_eq!(
            parse_partial_date_from_str("19620117"),
            Some(NaiveDate::from_ymd_opt(1962, 1, 17).unwrap())
        );
        assert_eq!(
            parse_partial_date_from_str("1986-04"),
            Some(NaiveDate::from_ymd_opt(1986, 4, 1).unwrap())
        );
        assert_eq!(
            parse_partial_date_from_str("198604"),
            Some(NaiveDate::from_ymd_opt(1986, 4, 1).unwrap())
        );
        assert_eq!(
            parse_partial_date_from_str("1986"),
            Some(NaiveDate::from_ymd_opt(1986, 1, 1).unwrap())
        );
    }
}
