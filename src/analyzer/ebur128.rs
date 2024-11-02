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

use ebur128::{energy_to_loudness, EbuR128, Mode};

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
    /// Number of gating blocks (for album gain calculation).
    pub gating_block_count: u64,
    /// Energy of the track (for album gain calculation).
    pub energy: f64,
}

impl EbuR128Result {
    /// Calculate ReplayGain 2.0 Track Gain.
    pub fn replaygain_track_gain(&self) -> f64 {
        REPLAYGAIN2_REFERENCE_LUFS - self.average_lufs
    }

    /// ReplayGain 2.0 Track Gain, formatted according to "Table 3: Metadata keys and value
    /// formatting" in the ["Metadata format" section in the ReplayGain 2.0 specification][rgmeta].
    ///
    /// [rgmeta]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Metadata_format
    pub fn replaygain_track_gain_string(&self) -> String {
        replaygain_gain_string(self.replaygain_track_gain())
    }

    /// ReplayGain 2.0 Track Peak, formatted according to "Table 3: Metadata keys and value
    /// formatting" in the ["Metadata format" section in the ReplayGain 2.0 specification][rgmeta].
    ///
    /// [rgmeta]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Metadata_format
    pub fn replaygain_track_peak_string(&self) -> String {
        replaygain_peak_string(self.peak)
    }
}

/// Result of the EBU R 128 album analysis.
#[derive(Debug, Clone)]
pub struct EbuR128AlbumResult {
    /// Measured loudness level of the audio files on the album.
    pub average_lufs: f64,
    /// Peak amplitude of the audio files on the album.
    pub peak: f64,
}

impl EbuR128AlbumResult {
    /// Calculate the ReplayGain 2.0 Album Peak and Album Gain from an iterator of `EbuR128Result`
    /// values.
    // FIXME: Remove this when anonymous lifetimes in `impl Trait` become stable.
    #[expect(single_use_lifetimes)]
    pub fn from_iter<'a>(
        results: impl Iterator<Item = &'a EbuR128Result>,
    ) -> Option<EbuR128AlbumResult> {
        let (album_peak, album_gating_block_count, album_energy) = results.fold(
            (0f64, 0u64, 0f64),
            |(album_peak, album_gating_block_count, album_energy), result| {
                (
                    album_peak.max(result.peak),
                    album_gating_block_count + result.gating_block_count,
                    album_energy + result.energy,
                )
            },
        );

        if album_gating_block_count == 0 {
            return None;
        }

        #[expect(clippy::cast_precision_loss)]
        let album_average_lufs =
            energy_to_loudness(album_energy / (album_gating_block_count as f64));

        Some(EbuR128AlbumResult {
            average_lufs: album_average_lufs,
            peak: album_peak,
        })
    }

    /// Calculate ReplayGain 2.0 Album Gain.
    pub fn replaygain_album_gain(&self) -> f64 {
        REPLAYGAIN2_REFERENCE_LUFS - self.average_lufs
    }

    /// ReplayGain 2.0 Album Gain, formatted according to "Table 3: Metadata keys and value
    /// formatting" in the ["Metadata format" section in the ReplayGain 2.0 specification][rgmeta].
    ///
    /// [rgmeta]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Metadata_format
    pub fn replaygain_album_gain_string(&self) -> String {
        replaygain_gain_string(self.replaygain_album_gain())
    }

    /// ReplayGain 2.0 Album Peak, formatted according to "Table 3: Metadata keys and value
    /// formatting" in the ["Metadata format" section in the ReplayGain 2.0 specification][rgmeta].
    ///
    /// [rgmeta]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Metadata_format
    pub fn replaygain_album_peak_string(&self) -> String {
        replaygain_peak_string(self.peak)
    }
}

/// Format an [`f64`] as a ReplayGain 2.0 Gain Value according to "Table 3: Metadata keys and
/// value formatting" in the ["Metadata format" section in the ReplayGain 2.0
/// specification][rgmeta].
///
/// [rgmeta]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Metadata_format
pub fn replaygain_gain_string(gain: f64) -> String {
    format!("{gain:.2} dB")
}

/// Format an [`f64`] as a ReplayGain 2.0 Peak Value according to "Table 3: Metadata keys and
/// value formatting" in the ["Metadata format" section in the ReplayGain 2.0
/// specification][rgmeta].
///
/// [rgmeta]: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification#Metadata_format
pub fn replaygain_peak_string(peak: f64) -> String {
    format!("{peak:.6}")
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
        let (gating_block_count, energy) =
            self.ebur128
                .gating_block_count_and_energy()
                .ok_or(AnalyzerError::Custom(
                    "gating block count and energy not available",
                ))?;
        Ok(EbuR128Result {
            average_lufs,
            peak,
            gating_block_count,
            energy,
        })
    }
}
