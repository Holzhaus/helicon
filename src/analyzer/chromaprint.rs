// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Chromaprint analysis.
//!
//! This calculates a compressed fingerprint compatrible with the output of [`fpcalc`][fpcalc].
//!
//! [fpcalc]: https://acoustid.org/chromaprint

use super::{Analyzer, AnalyzerError};
use crate::config::Config;
use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};

use symphonia::core::audio::Channels;
use symphonia::core::codecs::CodecParameters;

use rusty_chromaprint::{Configuration, FingerprintCompressor, Fingerprinter};

/// Chromaprint Analyzer.
#[allow(missing_debug_implementations)]
pub struct ChromaprintFingerprintAnalyzer {
    /// Fingerprinter Configuration,
    chromaprint_config: Configuration,
    /// Fingerprinter code.
    fingerprinter: Fingerprinter,
    /// Maximum stream size that will be analyzed.
    stream_size_max: usize,
    /// Current stream size that already was analyzed.
    stream_size: usize,
}

/// Analysis result of the Chromaprint analyzer.
#[derive(Debug, Clone)]
pub struct ChromaprintFingerprintResult {
    /// Analyzed duration.
    pub duration: usize,
    /// AcoudID fingerprint.
    pub fingerprint: Vec<u8>,
}

impl ChromaprintFingerprintResult {
    /// Return the chromaprint fingerprint as base64-encoded string (URL-style, no padding).
    pub fn fingerprint_string(&self) -> String {
        BASE64_URL_SAFE_NO_PAD.encode(&self.fingerprint)
    }
}

/// Maximum duration that will be analyzed.
const MAX_DURATION: usize = 120;

impl Analyzer for ChromaprintFingerprintAnalyzer {
    type Result = ChromaprintFingerprintResult;

    fn initialize(_config: &Config, codec_params: &CodecParameters) -> Result<Self, AnalyzerError> {
        let sample_rate = codec_params
            .sample_rate
            .ok_or(AnalyzerError::MissingSampleRate)?;
        let channels = codec_params
            .channels
            .map(Channels::count)
            .and_then(|channel_count| u32::try_from(channel_count).ok())
            .ok_or(AnalyzerError::MissingAudioChannels)?;

        let chromaprint_config = Configuration::preset_test2();
        let mut fingerprinter = Fingerprinter::new(&chromaprint_config);
        fingerprinter
            .start(sample_rate, channels)
            .map_err(|_err| AnalyzerError::ChromaprintResetError)?;
        let analyzer = Self {
            chromaprint_config,
            fingerprinter,
            stream_size_max: MAX_DURATION * usize::try_from(sample_rate).unwrap(),
            stream_size: 0,
        };
        Ok(analyzer)
    }

    fn feed(&mut self, samples: &[i16]) -> Result<(), AnalyzerError> {
        let remaining = self.stream_size_max - self.stream_size;
        let chunk_size = samples.len().min(remaining);
        self.stream_size += chunk_size;
        self.fingerprinter.consume(&samples[..chunk_size]);
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.stream_size >= self.stream_size_max
    }

    fn finalize(mut self) -> Result<Self::Result, AnalyzerError> {
        self.fingerprinter.finish();
        let raw_fingerprint = self.fingerprinter.fingerprint();
        let fingerprint =
            FingerprintCompressor::from(&self.chromaprint_config).compress(raw_fingerprint);
        Ok(Self::Result {
            duration: self.stream_size,
            fingerprint,
        })
    }
}
