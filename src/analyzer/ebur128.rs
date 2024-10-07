// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Loudness analysis according to the [EBU R 128 standard][ebur128].
//!
//! [ebur128]: https://en.wikipedia.org/wiki/EBU_R_128

use super::{Analyzer, AnalyzerError};
use crate::config::Config;

use symphonia::core::audio::Channels;
use symphonia::core::codecs::CodecParameters;

use ebur128::{EbuR128, Mode};

/// ReplayGain 2.0 Reference Gain
///
/// See the [ReplayGain 2.0 specification][rg2spec] for details.
///
/// [rg2spec]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Reference_level
const REPLAYGAIN2_REFERENCE_LUFS: f64 = -18.0;

/// EBU R128 Analyzer.
#[derive(Debug)]
pub struct EbuR128Analyzer {
    /// EBU R128 loudness analyzer.
    ebur128: EbuR128,
    /// Number of channels in the track (used for peak analysis).
    channels: u32,
    /// Chunk size in samples (usually 1s).
    chunk_size: usize,
}

/// Result of the EBU R 128 analysis.
#[derive(Debug, Clone)]
#[expect(missing_copy_implementations)]
pub struct EbuR128Result {
    /// Measured loudness level of the audio file.
    pub average_lufs: f64,
    /// Peak amplitude of the audio file.
    pub peak: f64,
}

impl EbuR128Result {
    /// Calculate ReplayGain 2.0 Track Gain.
    pub fn replaygain_track_gain(&self) -> f64 {
        REPLAYGAIN2_REFERENCE_LUFS - self.average_lufs
    }
}

/// Convert a dBFS value to a LUFS value.
///
/// See the [ReplayGain 2.0 specification][normalization] for details.
///
/// [normalization]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Loudness_normalization
#[expect(dead_code)]
fn dbfs_to_ratio(value: f64) -> f64 {
    10.0f64.powf(value / 20.0)
}

/// Convert a LUFS value to a dBFS value.
///
/// See the [ReplayGain 2.0 specification][normalization] for details.
///
/// [normalization]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Loudness_normalization
#[expect(dead_code)]
fn ratio_to_dbfs(value: f64) -> f64 {
    20.0 * value.log10()
}

impl Analyzer for EbuR128Analyzer {
    type Result = EbuR128Result;

    fn initialize(_config: &Config, codec_params: &CodecParameters) -> Result<Self, AnalyzerError> {
        let sample_rate = codec_params
            .sample_rate
            .ok_or(AnalyzerError::MissingSampleRate)?;
        let channel_count = codec_params
            .channels
            .map(Channels::count)
            .ok_or(AnalyzerError::MissingAudioChannels)?;

        let channels =
            u32::try_from(channel_count).map_err(|_| AnalyzerError::MissingAudioChannels)?;
        let chunk_size = usize::try_from(sample_rate)
            .map_err(|_err| AnalyzerError::MissingSampleRate)?
            * channel_count;

        let ebur128 = EbuR128::new(channels, sample_rate, Mode::all())?;
        let analyzer = Self {
            ebur128,
            channels,
            chunk_size,
        };
        Ok(analyzer)
    }

    fn feed(&mut self, samples: &[i16]) -> Result<(), AnalyzerError> {
        let samples: Vec<f32> = samples
            .iter()
            .map(|&sample| f32::from(sample) / f32::from(i16::MAX))
            .collect();
        for chunk in samples.chunks(self.chunk_size) {
            self.ebur128.add_frames_f32(chunk)?;
        }
        Ok(self.ebur128.loudness_global().map(|_| ())?)
    }

    fn is_complete(&self) -> bool {
        false
    }

    fn finalize(self) -> Result<Self::Result, AnalyzerError> {
        let average_lufs = self.ebur128.loudness_global()?;
        let peak = (0..self.channels)
            .map(|channel_index| self.ebur128.sample_peak(channel_index))
            .try_fold(0.0f64, |a, b| b.map(|b| a.max(b)))?;
        Ok(EbuR128Result { average_lufs, peak })
    }
}
