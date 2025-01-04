// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! SoundTouch BPM analysis.
//!
//! Detects the average Beats-per-minute (BPM) of the audio track using the [SoundTouch Audio
//! Processing Library][soundtouch].
//!
//! [soundtouch]: http://www.surina.net/soundtouch/

use super::{Analyzer, AnalyzerError};
use crate::config::Config;

use symphonia::core::audio::Channels;
use symphonia::core::codecs::CodecParameters;

use soundtouch::BPMDetect;

/// Chromaprint Analyzer.
#[allow(missing_debug_implementations)]
pub struct SoundTouchBpmAnalyzer {
    /// The [`BPMDetect`] struct which is doing the actual tempo analysis.
    bpm_detect: BPMDetect,
}

/// Analysis result of the Chromaprint analyzer.
#[allow(missing_copy_implementations)]
#[derive(Debug, Clone)]
pub struct SoundTouchBpmResult {
    /// Analyzed Beats per Minute (BPM).
    pub bpm: f32,
}

impl SoundTouchBpmResult {
    /// Return the BPM as a string.
    pub fn bpm_string(&self) -> String {
        format!("{bpm:.2}", bpm = self.bpm)
    }
}

impl Analyzer for SoundTouchBpmAnalyzer {
    type Result = SoundTouchBpmResult;

    fn initialize(_config: &Config, codec_params: &CodecParameters) -> Result<Self, AnalyzerError> {
        let sample_rate = codec_params
            .sample_rate
            .ok_or(AnalyzerError::MissingSampleRate)?;
        let num_channels = codec_params
            .channels
            .map(Channels::count)
            .and_then(|channel_count| u32::try_from(channel_count).ok())
            .ok_or(AnalyzerError::MissingAudioChannels)?;

        let bpm_detect = BPMDetect::new(num_channels, sample_rate);
        let analyzer = Self { bpm_detect };
        Ok(analyzer)
    }

    fn feed(&mut self, samples: &[i16]) -> Result<(), AnalyzerError> {
        let samples_float = samples
            .iter()
            .map(|&sample| sample.into())
            .collect::<Vec<f32>>();
        self.bpm_detect.input_samples(&samples_float);
        Ok(())
    }

    fn is_complete(&self) -> bool {
        // We need to read the entire file to calculate the average BPM.
        false
    }

    fn finalize(mut self) -> Result<Self::Result, AnalyzerError> {
        let bpm = self.bpm_detect.get_bpm();
        Ok(Self::Result { bpm })
    }
}
