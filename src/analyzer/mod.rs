// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Audio analysis.

use crate::config::{AnalyzerType, Config};

use std::path::Path;
use thiserror::Error;

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{CodecParameters, Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

mod chromaprint;
mod ebur128;
mod track_length;

use chromaprint::ChromaprintFingerprintAnalyzer;
use ebur128::EbuR128Analyzer;
use track_length::TrackLengthAnalyzer;

pub use ebur128::EbuR128AlbumResult;

/// An error during analysis.
#[derive(Error, Debug)]
pub enum AnalyzerError {
    /// I/O Error.
    #[error("Input/Output error ({:?})", .0)]
    IoError(#[from] std::io::Error),
    /// I/O Error.
    #[error("Symphonia error ({:?})", .0)]
    SymphoniaError(#[from] symphonia::core::errors::Error),
    /// No supported audio tracks.
    #[error("No supported audio tracks.")]
    NoSupportedAudioTracks,
    /// Chromaprint Error.
    #[error("Chromaprint reset error")]
    ChromaprintResetError,
    /// Missing Sample Rate.
    #[error("missing sample rate")]
    MissingSampleRate,
    /// Missing Audio Channels.
    #[error("missing audio channels")]
    MissingAudioChannels,
    /// Custom, analyzer-specific error.
    #[error("analyzer error: {0}")]
    Custom(&'static str),
    /// EBU R 128 analysis failed.
    #[error("ebur128 failure: {0}")]
    EbuR128Error(#[from] ::ebur128::Error),
}

/// Analyzer trait.
pub trait Analyzer
where
    Self: Sized,
{
    /// Analyzer result type.
    type Result;

    /// Initialize the analyzer.
    fn initialize(config: &Config, codec_params: &CodecParameters) -> Result<Self, AnalyzerError>;
    /// Feed samples into the analysis.
    fn feed(&mut self, samples: &[i16]) -> Result<(), AnalyzerError>;
    /// Returns `true` if the Analyzer is complete and does not need additional input.
    fn is_complete(&self) -> bool;
    /// Finalize the analysis and return the result.
    fn finalize(self) -> Result<Self::Result, AnalyzerError>;
}

/// Compound analyzer that runs multiple analyzers at the same time.
struct CompoundAnalyzer {
    /// The analyzers in this compound analyzer.
    analyzers: Vec<CompoundAnalyzerItem>,
    /// The results of the analyzers.
    results: CompoundAnalyzerResult,
}

/// An analyzer item for a [`CompoundAnalyzer`].
enum CompoundAnalyzerItem {
    /// Track Length Analyzer.
    TrackLength(Box<TrackLengthAnalyzer>),
    /// Chromaprint Fingerprint Analyzer.
    ChromaprintFingerprint(Box<ChromaprintFingerprintAnalyzer>),
    /// EBU R 128 Analyzer.
    EbuR128(Box<EbuR128Analyzer>),
}

impl CompoundAnalyzerItem {
    /// Initialize this analyzer or assign the error to the result struct if an error occurs.
    fn initialize_or_assign_result(
        analyzer_type: AnalyzerType,
        config: &Config,
        codec_params: &CodecParameters,
        result: &mut CompoundAnalyzerResult,
    ) -> Option<Self> {
        match analyzer_type {
            AnalyzerType::TrackLength => {
                match TrackLengthAnalyzer::initialize(config, codec_params) {
                    Ok(analyzer) => Some(Self::TrackLength(Box::from(analyzer))),
                    Err(err) => {
                        result.track_length = Some(Err(err));
                        None
                    }
                }
            }
            AnalyzerType::ChromaprintFingerprint => {
                match ChromaprintFingerprintAnalyzer::initialize(config, codec_params) {
                    Ok(analyzer) => Some(Self::ChromaprintFingerprint(Box::from(analyzer))),
                    Err(err) => {
                        result.chromaprint_fingerprint = Some(Err(err));
                        None
                    }
                }
            }
            AnalyzerType::EbuR128 => match EbuR128Analyzer::initialize(config, codec_params) {
                Ok(analyzer) => Some(Self::EbuR128(Box::from(analyzer))),
                Err(err) => {
                    result.ebur128 = Some(Err(err));
                    None
                }
            },
        }
    }

    /// Returns `true` if the Analyzer is complete and does not need additional input.
    fn is_complete(&self) -> bool {
        match self {
            Self::TrackLength(analyzer) => analyzer.is_complete(),
            Self::ChromaprintFingerprint(analyzer) => analyzer.is_complete(),
            Self::EbuR128(analyzer) => analyzer.is_complete(),
        }
    }

    /// Feed samples into the analyzer, or assign the error to the result struct if an error
    /// occurs.
    fn feed_or_assign_result(
        &mut self,
        samples: &[i16],
        result: &mut CompoundAnalyzerResult,
    ) -> bool {
        match self {
            Self::TrackLength(analyzer) => match analyzer.feed(samples) {
                Ok(()) => true,
                Err(err) => {
                    result.track_length = Some(Err(err));
                    false
                }
            },
            Self::ChromaprintFingerprint(analyzer) => match analyzer.feed(samples) {
                Ok(()) => true,
                Err(err) => {
                    result.chromaprint_fingerprint = Some(Err(err));
                    false
                }
            },
            Self::EbuR128(analyzer) => match analyzer.feed(samples) {
                Ok(()) => true,
                Err(err) => {
                    result.ebur128 = Some(Err(err));
                    false
                }
            },
        }
    }

    /// Finalize the analysis and assign the result to the result struct.
    fn finalize_and_assign_result(
        self,
        mut result: CompoundAnalyzerResult,
    ) -> CompoundAnalyzerResult {
        match self {
            Self::TrackLength(analyzer) => {
                result.track_length = Some(analyzer.finalize());
            }
            Self::ChromaprintFingerprint(analyzer) => {
                result.chromaprint_fingerprint = Some(analyzer.finalize());
            }
            Self::EbuR128(analyzer) => {
                result.ebur128 = Some(analyzer.finalize());
            }
        }
        result
    }
}

/// Compound result type that may contains results from all analyzers.
#[derive(Debug, Default)]
pub struct CompoundAnalyzerResult {
    /// Result of the track length analysis.
    pub track_length: Option<Result<<TrackLengthAnalyzer as Analyzer>::Result, AnalyzerError>>,
    /// Result of the chromaprint fingerprint analysis.
    pub chromaprint_fingerprint:
        Option<Result<<ChromaprintFingerprintAnalyzer as Analyzer>::Result, AnalyzerError>>,
    /// Result of the EBU R 128 analysis.
    pub ebur128: Option<Result<<EbuR128Analyzer as Analyzer>::Result, AnalyzerError>>,
}

impl Analyzer for CompoundAnalyzer {
    type Result = CompoundAnalyzerResult;

    fn initialize(config: &Config, codec_params: &CodecParameters) -> Result<Self, AnalyzerError> {
        let mut results = CompoundAnalyzerResult::default();
        let analyzers = config
            .analyzers
            .enabled
            .iter()
            .copied()
            .filter_map(|analyzer_type| {
                CompoundAnalyzerItem::initialize_or_assign_result(
                    analyzer_type,
                    config,
                    codec_params,
                    &mut results,
                )
            })
            .collect::<Vec<CompoundAnalyzerItem>>();

        Ok(Self { analyzers, results })
    }

    fn feed(&mut self, samples: &[i16]) -> Result<(), AnalyzerError> {
        self.analyzers
            .retain_mut(|analyzer| analyzer.feed_or_assign_result(samples, &mut self.results));
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.analyzers.iter().all(CompoundAnalyzerItem::is_complete)
    }

    fn finalize(self) -> Result<CompoundAnalyzerResult, AnalyzerError> {
        Ok(self
            .analyzers
            .into_iter()
            .fold(CompoundAnalyzerResult::default(), |results, analyzer| {
                analyzer.finalize_and_assign_result(results)
            }))
    }
}

/// Audio reader.
struct AudioReader {
    /// Audio format reader.
    format: Box<dyn FormatReader>,
    /// Audio decoder.
    decoder: Box<dyn Decoder>,
    /// Track ID.
    track_id: u32,
}

impl AudioReader {
    /// Create an audio reader from the given path.
    fn new(path: &impl AsRef<Path>) -> Result<Self, AnalyzerError> {
        let path = path.as_ref();
        let src = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(src), MediaSourceStreamOptions::default());

        let mut hint = Hint::new();

        #[expect(unused_results)]
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let meta_opts: MetadataOptions = MetadataOptions::default();
        let fmt_opts: FormatOptions = FormatOptions::default();

        let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(AnalyzerError::NoSupportedAudioTracks)?;

        let track_id = track.id;

        let dec_opts: DecoderOptions = DecoderOptions::default();

        let decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

        Ok(Self {
            format,
            decoder,
            track_id,
        })
    }

    /// Get the codec parameters.
    fn codec_params(&self) -> Option<&CodecParameters> {
        self.format
            .tracks()
            .iter()
            .find_map(|track| (track.id == self.track_id).then_some(&track.codec_params))
    }

    /// Read the next packet(s) that belongs to the current track, decode it and return a reference
    /// to the decoded audio buffer.
    fn next_buffer(&mut self) -> Result<AudioBufferRef<'_>, SymphoniaError> {
        let packet = loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                err => break err,
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            break Ok(packet);
        };
        packet.and_then(|pkt| self.decoder.decode(&pkt))
    }
}

/// Run an analysis.
pub fn analyze(
    config: &Config,
    path: impl AsRef<Path>,
) -> Result<CompoundAnalyzerResult, AnalyzerError> {
    log::debug!("Analyzing file: {}", path.as_ref().display());
    let mut reader = AudioReader::new(&path)?;
    let codec_params = reader
        .codec_params()
        .ok_or(AnalyzerError::NoSupportedAudioTracks)?;

    let mut analyzer = CompoundAnalyzer::initialize(config, codec_params)?;

    let mut sample_buf = None;
    while !analyzer.is_complete() {
        let audio_buf = match reader.next_buffer() {
            Ok(buffer) => buffer,
            Err(SymphoniaError::DecodeError(err)) => Err(SymphoniaError::DecodeError(err))?,
            Err(_) => break,
        };

        if sample_buf.is_none() {
            let spec = *audio_buf.spec();
            let duration = audio_buf.capacity() as u64;
            sample_buf = Some(SampleBuffer::<i16>::new(duration, spec));
        }

        if let Some(buf) = &mut sample_buf {
            buf.copy_interleaved_ref(audio_buf);
            analyzer.feed(buf.samples())?;
        }
    }

    analyzer.finalize()
}
