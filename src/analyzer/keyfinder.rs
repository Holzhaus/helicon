// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Musical key analyzer based on libkeyfinder.

use super::{Analyzer, AnalyzerError};
use crate::config::Config;
use crate::key::{MusicalKey, Tonic};

use symphonia::core::audio::Channels;
use symphonia::core::formats::Track;

use libkeyfinder_sys::{AudioData, KeyFinder, KeyFinderKey};

/// Keyfinder Analyzer for musical key detection.
#[allow(missing_debug_implementations)]
pub struct KeyFinderKeyAnalyzer {
    /// The audio data buffer.
    audio: AudioData,
}

/// Analysis result of the Keyfinder analyzer.
#[derive(Debug, Clone, Copy)]
pub struct KeyFinderKeyResult {
    /// Detected musical key.
    pub key: MusicalKey,
}

impl From<KeyFinderKey> for MusicalKey {
    fn from(key: KeyFinderKey) -> Self {
        use Tonic::{AFlat, BFlat, DFlat, EFlat, GFlat, A, B, C, D, E, F, G};
        match key {
            KeyFinderKey::CMajor => Self::Major(C),
            KeyFinderKey::CMinor => Self::Minor(C),
            KeyFinderKey::DFlatMajor => Self::Major(DFlat),
            KeyFinderKey::DFlatMinor => Self::Minor(DFlat),
            KeyFinderKey::DMajor => Self::Major(D),
            KeyFinderKey::DMinor => Self::Minor(D),
            KeyFinderKey::EFlatMajor => Self::Major(EFlat),
            KeyFinderKey::EFlatMinor => Self::Minor(EFlat),
            KeyFinderKey::EMajor => Self::Major(E),
            KeyFinderKey::EMinor => Self::Minor(E),
            KeyFinderKey::FMajor => Self::Major(F),
            KeyFinderKey::FMinor => Self::Minor(F),
            KeyFinderKey::GFlatMajor => Self::Major(GFlat),
            KeyFinderKey::GFlatMinor => Self::Minor(GFlat),
            KeyFinderKey::GMajor => Self::Major(G),
            KeyFinderKey::GMinor => Self::Minor(G),
            KeyFinderKey::AFlatMajor => Self::Major(AFlat),
            KeyFinderKey::AFlatMinor => Self::Minor(AFlat),
            KeyFinderKey::AMajor => Self::Major(A),
            KeyFinderKey::AMinor => Self::Minor(A),
            KeyFinderKey::BFlatMajor => Self::Major(BFlat),
            KeyFinderKey::BFlatMinor => Self::Minor(BFlat),
            KeyFinderKey::BMajor => Self::Major(B),
            KeyFinderKey::BMinor => Self::Minor(B),
            KeyFinderKey::Silence => Self::Silence,
        }
    }
}

impl Analyzer for KeyFinderKeyAnalyzer {
    type Result = KeyFinderKeyResult;

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
            .ok_or(AnalyzerError::MissingAudioChannels)?;

        let mut audio = AudioData::new();
        audio.set_frame_rate(sample_rate);
        audio.set_channels(
            u32::try_from(num_channels).map_err(|_| AnalyzerError::MissingAudioChannels)?,
        );

        let analyzer = Self { audio };
        Ok(analyzer)
    }

    fn feed(&mut self, samples: &[f32]) -> Result<(), AnalyzerError> {
        self.audio.extend(samples.to_owned());
        Ok(())
    }

    fn is_complete(&self) -> bool {
        // We need to read the entire file to accurately detect the key.
        false
    }

    fn finalize(self) -> Result<Self::Result, AnalyzerError> {
        let mut key_finder = KeyFinder::new();
        let key = key_finder.key_of_audio(&self.audio);

        Ok(Self::Result { key: key.into() })
    }
}
