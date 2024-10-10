// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
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

use rusty_chromaprint::{Configuration, Fingerprinter};

/// Chromaprint Analyzer.
#[allow(missing_debug_implementations)]
pub struct ChromaprintFingerprintAnalyzer {
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
        let fingerprint = compressor::compress(self.fingerprinter.fingerprint(), 1);
        Ok(Self::Result {
            duration: self.stream_size,
            fingerprint,
        })
    }
}

mod compressor {
    //! Fingerprint compressor module.

    /// Number of "normal" bits.
    const NORMAL_BITS: u8 = 3;
    /// Maximum "normal" value above which a value becomes "exceptional".
    const MAX_NORMAL_VALUE: u8 = (1 << NORMAL_BITS) - 1;

    /// Turns an object (e.g. an `u32`) over an iterator of bits.
    trait IntoBitIterator {
        /// Converts the item into an an iterator over its bits.
        fn into_bit_iter(self) -> impl Iterator<Item = bool>;
    }

    impl IntoBitIterator for u32 {
        fn into_bit_iter(self) -> impl Iterator<Item = bool> {
            (0..Self::BITS).map(move |index| ((self >> index) & 1) == 1)
        }
    }

    /// Compress a sub-fingerprint.
    fn compress_subfingerprint(subfingerprint: u32) -> impl Iterator<Item = (u8, Option<u8>)> {
        subfingerprint
            .into_bit_iter()
            .enumerate()
            .filter_map(|(bit_index, is_bit_set)| {
                is_bit_set.then_some(u8::try_from(bit_index).unwrap())
            })
            .scan(0, |last_bit_index, bit_index| {
                let value = bit_index - *last_bit_index;
                let result = if value >= MAX_NORMAL_VALUE {
                    (MAX_NORMAL_VALUE, Some(value - MAX_NORMAL_VALUE))
                } else {
                    (value, None)
                };

                *last_bit_index = bit_index;
                Some(result)
            })
            .chain(std::iter::once((0, None)))
    }

    /// Compress the fingerprint.
    pub fn compress(fingerprint: &[u32], algorithm: u32) -> Vec<u8> {
        let size = fingerprint.len();
        let (normal_bits, exceptional_bits) = fingerprint
            .iter()
            .scan(0, |last_subfp, current_subfp| {
                let value = current_subfp ^ *last_subfp;
                *last_subfp = *current_subfp;
                Some(value)
            })
            .flat_map(compress_subfingerprint)
            .fold(
                (
                    Vec::<u8>::with_capacity(size),
                    Vec::<u8>::with_capacity(size),
                ),
                |(mut normal_bits, mut exceptional_bits), (normal_value, exceptional_value)| {
                    normal_bits.push(normal_value);
                    if let Some(exceptional_value) = exceptional_value {
                        exceptional_bits.push(exceptional_value);
                    }
                    (normal_bits, exceptional_bits)
                },
            );

        let header_size = 4;
        let normal_size = packed_intn_array_len(normal_bits.len(), 3);
        let exceptional_size = packed_intn_array_len(exceptional_bits.len(), 5);
        let expected_size = header_size + normal_size + exceptional_size;

        #[allow(clippy::cast_possible_truncation)]
        let output = [
            (algorithm & 0xFF) as u8,
            ((size >> 16) & 0xFF) as u8,
            ((size >> 8) & 0xFF) as u8,
            (size & 0xFF) as u8,
        ];

        let output = output
            .into_iter()
            .chain(iter_packed_intn_array::<3>(&normal_bits))
            .chain(iter_packed_intn_array::<5>(&exceptional_bits))
            .collect::<Vec<u8>>();
        debug_assert_eq!(output.len(), expected_size);
        output
    }

    /// Calculate the size of a packed Int<N> array.
    fn packed_intn_array_len(array_len: usize, n: usize) -> usize {
        (array_len * n + 7) / 8
    }

    /// Iterate bytes as packed Int<N> array.
    fn iter_packed_intn_array<const N: usize>(array: &[u8]) -> impl Iterator<Item = u8> + '_ {
        let mask = (0xFF << (8 - N)) >> (8 - N);
        array.chunks(8).flat_map(move |slice| {
            let (size, result) = slice.iter().map(|s| s & mask).enumerate().fold(
                (0, [0u8; N]),
                |(_, mut result), (i, bits)| {
                    let rightmost_bit_index = i * N;
                    let leftmost_bit_index = rightmost_bit_index + N - 1;

                    let right_byte = rightmost_bit_index / 8;
                    let left_byte = leftmost_bit_index / 8;

                    result[right_byte] |= bits << (rightmost_bit_index % 8);
                    if left_byte != right_byte {
                        result[left_byte] |= bits >> ((8 - (rightmost_bit_index % 8)) % 8);
                    }

                    (left_byte + 1, result)
                },
            );
            result.into_iter().take(size)
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const BITBUFFER: [u8; 9] = [
            0b1010_1010,
            0b0011_0011,
            0b1100_1100,
            0b1100_0111,
            0b0101_0101,
            0b1100_1100,
            0b1010_1010,
            0b0000_0000,
            0b1111_1111,
        ];

        #[test]
        fn test_iter_packed_int3_array() {
            let int3packed = iter_packed_intn_array::<3>(&BITBUFFER).collect::<Vec<u8>>();
            assert_eq!(
                &int3packed,
                &[0b0001_1010, 0b0101_1111, 0b0000_1010, 0b0000_0111]
            );
        }

        #[test]
        fn test_iter_packed_int5_array() {
            let int5packed = iter_packed_intn_array::<5>(&BITBUFFER).collect::<Vec<u8>>();
            assert_eq!(
                &int5packed,
                &[
                    0b0110_1010,
                    0b1011_0010,
                    0b0101_0011,
                    0b1001_1001,
                    0b0000_0010,
                    0b0001_1111
                ]
            );
        }
    }
}
