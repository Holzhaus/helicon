// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
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
use float_eq::float_eq;

use symphonia::core::codecs::CodecParameters;

/// Track Length Analyzer.
#[derive(Debug)]
#[expect(missing_copy_implementations)]
pub struct TrackLengthAnalyzer {
    /// The track length (already determined during initialization).
    track_length: TimeDelta,
}

/// Number of nanoseconds per second.
const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;

impl Analyzer for TrackLengthAnalyzer {
    type Result = TimeDelta;

    fn initialize(_config: &Config, codec_params: &CodecParameters) -> Result<Self, AnalyzerError> {
        let track_length = codec_params
            .time_base
            .zip(codec_params.n_frames)
            .map(|(time_base, n_frames)| time_base.calc_time(n_frames))
            .and_then(|time| {
                i64::try_from(time.seconds)
                    .ok()
                    .zip(f64_to_u32((time.frac * NANOSECONDS_PER_SECOND).trunc()))
            })
            .and_then(|(secs, nanos)| TimeDelta::new(secs, nanos))
            .ok_or(AnalyzerError::Custom("Failed to calculate track length"))?;
        Ok(Self { track_length })
    }

    fn feed(&mut self, _samples: &[i16]) -> Result<(), AnalyzerError> {
        Ok(())
    }

    fn is_complete(&self) -> bool {
        true
    }

    fn finalize(self) -> Result<TimeDelta, AnalyzerError> {
        Ok(self.track_length)
    }
}

/// Convert an `f64` to `u32` (if possible).
#[expect(clippy::cast_sign_loss)]
#[expect(clippy::cast_possible_truncation)]
fn f64_to_u32(value: f64) -> Option<u32> {
    let intvalue = value as u32;
    if float_eq!(f64::from(intvalue), value, abs <= 0.000_1) {
        Some(intvalue)
    } else {
        None
    }
}
