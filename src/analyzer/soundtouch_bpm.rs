// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
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

#![cfg(feature = "soundtouch")]

use super::{Analyzer, AnalyzerError};
use crate::config::Config;

use symphonia::core::audio::Channels;
use symphonia::core::formats::Track;

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

    fn initialize(_config: &Config, track: &Track) -> Result<Self, AnalyzerError> {
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or(AnalyzerError::NoSupportedAudioTracks)?;
        let sample_rate = params.sample_rate.ok_or(AnalyzerError::MissingSampleRate)?;
        let num_channels = params
            .channels
            .as_ref()
            .map(Channels::count)
            .and_then(|channel_count| u32::try_from(channel_count).ok())
            .ok_or(AnalyzerError::MissingAudioChannels)?;

        let bpm_detect = BPMDetect::new(num_channels, sample_rate);
        let analyzer = Self { bpm_detect };
        Ok(analyzer)
    }

    fn feed(&mut self, samples: &[f32]) -> Result<(), AnalyzerError> {
        self.bpm_detect.input_samples(samples);
        Ok(())
    }

    fn is_complete(&self) -> bool {
        // We need to read the entire file to calculate the average BPM.
        false
    }

    fn finalize(mut self) -> Result<Self::Result, AnalyzerError> {
        let bpm = self.bpm_detect.get_bpm();
        if bpm == 0.0 {
            Err(AnalyzerError::Custom("No Beats detected"))
        } else {
            Ok(Self::Result { bpm })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use symphonia::core::codecs::audio::AudioCodecParameters;
    use symphonia::core::codecs::CodecParameters;

    const SAMPLE_RATE: u32 = 44100;

    fn track(sample_rate: Option<u32>, channels: Option<Channels>) -> Track {
        let mut params = AudioCodecParameters::new();
        if let Some(rate) = sample_rate {
            let _ = params.with_sample_rate(rate);
        }
        if let Some(channels) = channels {
            let _ = params.with_channels(channels);
        }
        let mut track = Track::new(0);
        let _ = track.with_codec_params(CodecParameters::Audio(params));
        track
    }

    /// Generate a mono click track with decaying noise bursts at the given tempo.
    fn click_track(bpm: u32, seconds: u32) -> Vec<f32> {
        let total = (SAMPLE_RATE * seconds) as usize;
        let beat_len = (60 * SAMPLE_RATE / bpm) as usize;
        let burst_len: u16 = 2000;
        let mut state = 12345u32;
        (0..total)
            .map(|i| {
                let pos = i % beat_len;
                match u16::try_from(pos) {
                    Ok(pos) if pos < burst_len => {
                        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        let noise = f32::from((state >> 16) as u16) / 32768.0 - 1.0;
                        noise * (1.0 - f32::from(pos) / f32::from(burst_len))
                    }
                    _ => 0.0,
                }
            })
            .collect()
    }

    #[test]
    fn test_bpm_string_formatting() {
        assert_eq!(SoundTouchBpmResult { bpm: 120.0 }.bpm_string(), "120.00");
        assert_eq!(SoundTouchBpmResult { bpm: 127.456 }.bpm_string(), "127.46");
    }

    #[test]
    fn test_initialize_without_codec_params() {
        let result = SoundTouchBpmAnalyzer::initialize(&Config::default(), &Track::new(0));
        assert!(matches!(result, Err(AnalyzerError::NoSupportedAudioTracks)));
    }

    #[test]
    fn test_initialize_missing_sample_rate() {
        let track = track(None, Some(Channels::Discrete(1)));
        let result = SoundTouchBpmAnalyzer::initialize(&Config::default(), &track);
        assert!(matches!(result, Err(AnalyzerError::MissingSampleRate)));
    }

    #[test]
    fn test_initialize_missing_channels() {
        let track = track(Some(SAMPLE_RATE), None);
        let result = SoundTouchBpmAnalyzer::initialize(&Config::default(), &track);
        assert!(matches!(result, Err(AnalyzerError::MissingAudioChannels)));
    }

    #[test]
    fn test_is_never_complete() {
        let track = track(Some(SAMPLE_RATE), Some(Channels::Discrete(1)));
        let mut analyzer = SoundTouchBpmAnalyzer::initialize(&Config::default(), &track).unwrap();
        assert!(!analyzer.is_complete());
        analyzer.feed(&click_track(120, 1)).unwrap();
        assert!(!analyzer.is_complete());
    }

    #[test]
    fn test_silence_has_no_beats() {
        let track = track(Some(SAMPLE_RATE), Some(Channels::Discrete(1)));
        let mut analyzer = SoundTouchBpmAnalyzer::initialize(&Config::default(), &track).unwrap();
        analyzer
            .feed(&vec![0.0; (SAMPLE_RATE * 10) as usize])
            .unwrap();
        assert!(matches!(
            analyzer.finalize(),
            Err(AnalyzerError::Custom("No Beats detected"))
        ));
    }

    #[test]
    fn test_detects_click_track_bpm() {
        for expected in [100_u16, 120, 140] {
            let track = track(Some(SAMPLE_RATE), Some(Channels::Discrete(1)));
            let mut analyzer =
                SoundTouchBpmAnalyzer::initialize(&Config::default(), &track).unwrap();
            for chunk in click_track(u32::from(expected), 30).chunks(4096) {
                analyzer.feed(chunk).unwrap();
            }
            let result = analyzer.finalize().unwrap();
            assert!(
                (result.bpm - f32::from(expected)).abs() < 1.5,
                "expected ~{expected}, got {}",
                result.bpm
            );
        }
    }
}
