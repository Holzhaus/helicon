// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Track Length analysis.

use super::{Analyzer, AnalyzerError};
use crate::config::Config;
use chrono::TimeDelta;
use symphonia::core::formats::Track;
use symphonia::core::units::Time;

/// Track Length Analyzer.
#[derive(Debug)]
#[expect(missing_copy_implementations)]
pub struct TrackLengthAnalyzer {
    /// The track length (already determined during initialization).
    track_length: TimeDelta,
}

impl Analyzer for TrackLengthAnalyzer {
    type Result = TimeDelta;

    fn initialize(_config: &Config, track: &Track) -> Result<Self, AnalyzerError> {
        let track_length = track
            .time_base
            .zip(
                track
                    .duration
                    .and_then(|duration| track.start_ts.checked_add(duration)),
            )
            .and_then(|(time_base, end_ts)| time_base.calc_time(end_ts))
            .as_ref()
            .map(Time::parts)
            .and_then(|(secs, nanos)| TimeDelta::new(secs, nanos))
            .ok_or(AnalyzerError::Custom("Failed to calculate track length"))?;
        Ok(Self { track_length })
    }

    fn feed(&mut self, _samples: &[f32]) -> Result<(), AnalyzerError> {
        Ok(())
    }

    fn is_complete(&self) -> bool {
        true
    }

    fn finalize(self) -> Result<TimeDelta, AnalyzerError> {
        Ok(self.track_length)
    }
}
