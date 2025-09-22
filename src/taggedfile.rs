// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! The [`TaggedFile`] struct represents a file that contains tags.

use crate::analyzer::CompoundAnalyzerResult;
use crate::media::MediaLike;
use crate::release::ReleaseLike;
use crate::tag::{read_tags_from_path, Tag, TagKey, TagType};
use crate::track::{AnalyzedTrackMetadata, InvolvedPerson, TrackLike};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::mem;
use std::path::{Path, PathBuf};

/// A tagged file that contains zero or more tags.
pub struct TaggedFile {
    /// Path of the file.
    pub path: PathBuf,
    /// Tags that are present in the file.
    content: Vec<Box<dyn Tag>>,
    /// Analysis results.
    pub analysis_results: Option<CompoundAnalyzerResult>,
}

impl fmt::Debug for TaggedFile {
    #[expect(unused_results)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let mut s = f.debug_tuple("TaggedFile");
        for tag in self.tags() {
            s.field(&tag.tag_type());
        }
        s.finish()
    }
}

impl TaggedFile {
    /// Create a new tagged file with an empty path from the given tag.
    #[cfg(test)]
    #[must_use]
    pub fn new(content: Vec<Box<dyn Tag>>) -> Self {
        TaggedFile {
            path: PathBuf::new(),
            content,
            analysis_results: None,
        }
    }

    /// Convert tags in this file.
    ///
    /// Currently, this always converts all ID3v2.x tags to ID3v2.3.
    ///
    /// # Panics
    ///
    /// This function may panic with the tag type indicates an ID3v2.x tag but the
    /// `Tag::maybe_as_id3v2_mut()` function returns `None`, which constitutes a programming error.
    pub fn convert_tags(&mut self) {
        #[cfg(feature = "id3")]
        {
            let (has_id3v22, has_id3v23, has_id3v24) = self.content.iter().fold(
                (false, false, false),
                |(mut has_id3v22, mut has_id3v23, mut has_id3v24), tag| {
                    #[allow(clippy::match_wildcard_for_single_variants)]
                    match tag.tag_type() {
                        TagType::ID3v22 => has_id3v22 = true,
                        TagType::ID3v23 => has_id3v23 = true,
                        TagType::ID3v24 => has_id3v24 = true,
                        _ => (),
                    }
                    (has_id3v22, has_id3v23, has_id3v24)
                },
            );
            let capacity = self.content.len();
            let old_content = mem::replace(&mut self.content, Vec::with_capacity(capacity));
            if has_id3v23 {
                old_content
                    .into_iter()
                    .filter(|tag| {
                        tag.tag_type() != TagType::ID3v22 && tag.tag_type() != TagType::ID3v24
                    })
                    .for_each(|tag| self.content.push(tag));
            } else if has_id3v24 {
                old_content
                    .into_iter()
                    .filter_map(|mut tag| {
                        if tag.tag_type() == TagType::ID3v24 {
                            tag.maybe_as_id3v2_mut()
                                .expect(
                                    "ID3 tags should always return `Some()` for `maybe_as_id3()`",
                                )
                                .migrate_to(id3::Version::Id3v23);
                            Some(tag)
                        } else if tag.tag_type() == TagType::ID3v22 {
                            None
                        } else {
                            Some(tag)
                        }
                    })
                    .for_each(|tag| self.content.push(tag));
            } else if has_id3v22 {
                old_content
                    .into_iter()
                    .map(|mut tag| {
                        if tag.tag_type() == TagType::ID3v22 {
                            tag.maybe_as_id3v2_mut()
                                .expect(
                                    "ID3 tags should always return `Some()` for `maybe_as_id3()`",
                                )
                                .migrate_to(id3::Version::Id3v23);
                        }
                        tag
                    })
                    .for_each(|tag| self.content.push(tag));
            }
        }
    }

    /// Creates a [`TaggedFile`] from the path.
    ///
    /// # Errors
    ///
    /// Returns an error in case the file at the given path does not exist or is unsupported.
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        read_tags_from_path(path.as_ref()).map(|content| Self {
            path: path.as_ref().to_path_buf(),
            content,
            analysis_results: None,
        })
    }

    /// Set additional analysis results for this file.
    #[must_use]
    pub fn with_analysis_results(
        mut self,
        analysis_results: Option<CompoundAnalyzerResult>,
    ) -> Self {
        self.analysis_results = analysis_results;
        self
    }

    /// Returns zero or more [`Tag`] objects.
    #[must_use]
    pub fn tags(&self) -> &[Box<dyn Tag>] {
        &self.content
    }

    /// Yields all values for the given [`TagKey`].
    pub fn tag_values<'a>(&'a self, key: &'a TagKey) -> impl Iterator<Item = Cow<'a, str>> {
        self.tags().iter().filter_map(move |tag| tag.get(key))
    }

    /// Yields all values for the given [`TagKey`].
    #[expect(clippy::needless_pass_by_value)]
    pub fn set_tag_value(&mut self, key: &TagKey, value: Option<Cow<'_, str>>) {
        self.content
            .iter_mut()
            .for_each(|tag| tag.set_or_clear(key, value.clone()));
    }

    /// Yields all values for the given [`TagKey`].
    pub fn set_tag_values(&mut self, key: &TagKey, values: &[Cow<'_, str>]) {
        self.content
            .iter_mut()
            .for_each(|tag| tag.set_multiple(key, values));
    }

    /// Returns the first value for the given [`TagKey`].
    #[must_use]
    pub fn first_tag_value<'a>(&'a self, key: &'a TagKey) -> Option<Cow<'a, str>> {
        self.tag_values(key).next()
    }

    /// Assign metadata from a `ReleaseLike` struct (e.g. a MusicBrainz release).
    pub fn assign_tags_from_release(&mut self, release: &impl ReleaseLike) {
        self.set_tag_value(&TagKey::Album, release.release_title());
        self.set_tag_value(&TagKey::AlbumArtist, release.release_artist());
        self.set_tag_value(
            &TagKey::AlbumArtistSortOrder,
            release.release_artist_sort_order(),
        );
        self.set_tag_value(&TagKey::AlbumSortOrder, release.release_sort_order());
        self.set_tag_value(&TagKey::Asin, release.asin());
        self.set_tag_value(&TagKey::Barcode, release.barcode());
        self.set_tag_value(&TagKey::CatalogNumber, release.catalog_number());
        self.set_tag_value(&TagKey::Compilation, release.compilation());
        self.set_tag_value(&TagKey::Grouping, release.grouping());
        self.set_tag_value(
            &TagKey::MusicBrainzReleaseArtistId,
            release.musicbrainz_release_artist_id(),
        );
        self.set_tag_value(
            &TagKey::MusicBrainzReleaseGroupId,
            release.musicbrainz_release_group_id(),
        );
        self.set_tag_value(
            &TagKey::MusicBrainzReleaseId,
            release.musicbrainz_release_id(),
        );
        self.set_tag_value(&TagKey::RecordLabel, release.record_label());
        self.set_tag_value(&TagKey::ReleaseCountry, release.release_country());
        self.set_tag_value(&TagKey::ReleaseDate, release.release_date());
        self.set_tag_value(&TagKey::ReleaseYear, release.release_year());
        self.set_tag_value(&TagKey::ReleaseStatus, release.release_status());
        self.set_tag_value(&TagKey::ReleaseType, release.release_type());
        self.set_tag_value(&TagKey::Script, release.script());
        self.set_tag_value(&TagKey::TotalDiscs, release.total_discs());
    }

    /// Assign metadata from a `MediaLike` struct (e.g. a disc of a MusicBrainz release).
    pub fn assign_tags_from_media(&mut self, media: &impl MediaLike) {
        //self.set_tag_value(&TagKey::DiscNumber, .media_title());
        self.set_tag_value(&TagKey::DiscSubtitle, media.media_title());
        self.set_tag_value(
            &TagKey::GaplessPlayback,
            media
                .gapless_playback()
                .map(|v| Cow::from(if v { "1" } else { "0" })),
        );
        self.set_tag_value(&TagKey::Media, media.media_format());
        self.set_tag_value(&TagKey::MusicBrainzDiscId, media.musicbrainz_disc_id());
        self.set_tag_value(
            &TagKey::TotalTracks,
            media
                .media_track_count()
                .map(|count| Cow::from(format!("{count}"))),
        );
    }

    /// Assign metadata from another `TrackLike` struct (e.g. a MusicBrainz track).
    pub fn assign_tags_from_track(&mut self, track: &impl TrackLike) {
        self.set_tag_value(&TagKey::AcoustId, track.acoustid());
        let acoustid_fingerprint = self
            .analyzed_metadata()
            .acoustid_fingerprint()
            .map(|value| Cow::from(value.to_string()));
        self.set_tag_value(&TagKey::AcoustIdFingerprint, acoustid_fingerprint);
        self.set_tag_values(
            &TagKey::Arranger,
            track.arranger().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(&TagKey::Artist, track.track_artist());
        self.set_tag_value(&TagKey::ArtistSortOrder, track.track_artist_sort_order());
        self.set_tag_value(&TagKey::Artists, track.track_artist());
        self.set_tag_value(&TagKey::Bpm, track.bpm());
        self.set_tag_value(&TagKey::Comment, track.comment());
        self.set_tag_values(
            &TagKey::Composer,
            track.composer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(&TagKey::ComposerSortOrder, track.composer_sort_order());
        self.set_tag_values(
            &TagKey::Conductor,
            track.conductor().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(&TagKey::Copyright, track.copyright());
        self.set_tag_values(
            &TagKey::Director,
            track.director().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_values(
            &TagKey::DjMixer,
            track.dj_mixer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(&TagKey::EncodedBy, track.encoded_by());
        self.set_tag_value(&TagKey::EncoderSettings, track.encoder_settings());
        self.set_tag_values(
            &TagKey::Engineer,
            track.engineer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_values(&TagKey::Genre, track.genre().collect::<Vec<_>>().as_slice());
        self.set_tag_value(&TagKey::InitialKey, track.initial_key());
        self.set_tag_values(&TagKey::Isrc, track.isrc().collect::<Vec<_>>().as_slice());
        self.set_tag_value(&TagKey::Language, track.language());
        self.set_tag_value(&TagKey::License, track.license());
        self.set_tag_values(
            &TagKey::Lyricist,
            track.lyricist().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(&TagKey::Lyrics, track.lyrics());
        self.set_tag_values(&TagKey::Mixer, track.mixer().collect::<Vec<_>>().as_slice());
        self.set_tag_value(&TagKey::Mood, track.mood());
        self.set_tag_value(&TagKey::Movement, track.movement());
        self.set_tag_value(&TagKey::MovementCount, track.movement_count());
        self.set_tag_value(&TagKey::MovementNumber, track.movement_number());
        self.set_tag_value(&TagKey::MusicBrainzArtistId, track.musicbrainz_artist_id());
        self.set_tag_value(
            &TagKey::MusicBrainzOriginalArtistId,
            track.musicbrainz_original_artist_id(),
        );
        self.set_tag_value(
            &TagKey::MusicBrainzOriginalReleaseId,
            track.musicbrainz_original_release_id(),
        );
        self.set_tag_value(
            &TagKey::MusicBrainzRecordingId,
            track.musicbrainz_recording_id(),
        );
        self.set_tag_value(&TagKey::MusicBrainzTrackId, track.musicbrainz_track_id());
        self.set_tag_value(&TagKey::MusicBrainzTrmId, track.musicbrainz_trm_id());
        self.set_tag_value(&TagKey::MusicBrainzWorkId, track.musicbrainz_work_id());
        self.set_tag_value(&TagKey::MusicIpFingerprint, track.musicip_fingerprint());
        self.set_tag_value(&TagKey::MusicIpPuid, track.musicip_puid());
        self.set_tag_value(&TagKey::OriginalAlbum, track.original_album());
        self.set_tag_value(&TagKey::OriginalArtist, track.original_artist());
        self.set_tag_value(&TagKey::OriginalFilename, track.original_filename());
        self.set_tag_value(&TagKey::OriginalReleaseDate, track.original_release_date());
        self.set_tag_value(&TagKey::OriginalReleaseYear, track.original_release_year());

        self.content
            .iter_mut()
            .for_each(|tag| tag.set_or_clear(&TagKey::Performers, None));
        let mut performers = HashMap::new();
        for performer in track.performers().into_iter().flatten() {
            performers
                .entry(performer.involvement)
                .or_insert_with(Vec::new)
                .push(performer.involvee);
        }
        for (involvement, involvees) in performers.drain() {
            self.content.iter_mut().for_each(|tag| {
                tag.set_multiple(
                    &TagKey::Performer(involvement.to_string()),
                    involvees.as_slice(),
                );
            });
        }

        self.set_tag_values(
            &TagKey::Producer,
            track.producer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(&TagKey::Rating, track.rating());
        self.set_tag_values(
            &TagKey::Remixer,
            track.remixer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(
            &TagKey::ReplayGainReferenceLoudness,
            track.replay_gain_reference_loudness(),
        );
        let replay_gain_track_gain = self
            .analyzed_metadata()
            .replay_gain_track_gain()
            .map(|value| Cow::from(value.to_string()));
        self.set_tag_value(&TagKey::ReplayGainTrackGain, replay_gain_track_gain);
        let replay_gain_track_peak = self
            .analyzed_metadata()
            .replay_gain_track_peak()
            .map(|value| Cow::from(value.to_string()));
        self.set_tag_value(&TagKey::ReplayGainTrackPeak, replay_gain_track_peak);
        let replay_gain_track_range = self
            .analyzed_metadata()
            .replay_gain_track_range()
            .map(|value| Cow::from(value.to_string()));
        self.set_tag_value(&TagKey::ReplayGainTrackRange, replay_gain_track_range);
        let bpm_analyzed = self
            .analyzed_metadata()
            .bpm()
            .map(|value| Cow::from(value.to_string()));
        self.set_tag_value(&TagKey::Bpm, bpm_analyzed);
        self.set_tag_value(&TagKey::TrackNumber, track.track_number());
        self.set_tag_value(&TagKey::TrackTitle, track.track_title());
        self.set_tag_value(&TagKey::TrackTitleSortOrder, track.track_title_sort_order());
        self.set_tag_value(&TagKey::ArtistWebsite, track.artist_website());
        self.set_tag_value(&TagKey::WorkTitle, track.work_title());
        self.set_tag_values(
            &TagKey::Writer,
            track.writer().collect::<Vec<_>>().as_slice(),
        );
    }

    /// Write tags to file.
    ///
    /// # Errors
    ///
    /// Returns an error if writing any underlying tag fails.
    pub fn write_tags(&mut self) -> crate::Result<()> {
        for tag in &mut self.content {
            tag.write(self.path.as_path())?;
        }

        Ok(())
    }
}

impl PartialEq for TaggedFile {
    fn eq(&self, other: &Self) -> bool {
        self.path.as_path().eq(other.path.as_path())
    }
}

impl Eq for TaggedFile {}

impl Ord for TaggedFile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.as_path().cmp(other.path.as_path())
    }
}

impl PartialOrd for TaggedFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TrackLike for TaggedFile {
    fn acoustid(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::AcoustId)
    }

    fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::AcoustIdFingerprint)
    }

    fn arranger(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Arranger)
    }

    fn track_artist(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Artist)
            .or_else(|| self.first_tag_value(&TagKey::Artists))
            .or_else(|| self.first_tag_value(&TagKey::AlbumArtist))
    }

    fn track_artist_sort_order(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ArtistSortOrder)
    }

    fn bpm(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Bpm)
    }

    fn comment(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Comment)
    }

    fn composer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Composer)
    }

    fn composer_sort_order(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ComposerSortOrder)
    }

    fn conductor(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Conductor)
    }

    fn copyright(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Copyright)
    }

    fn director(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Director)
    }

    fn dj_mixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::DjMixer)
    }

    fn encoded_by(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::EncodedBy)
    }

    fn encoder_settings(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::EncoderSettings)
    }

    fn engineer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Engineer)
    }

    fn genre(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Genre)
    }

    fn initial_key(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::InitialKey)
    }

    fn isrc(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Isrc)
    }

    fn language(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Language)
    }

    fn license(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::License)
    }

    fn lyricist(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Lyricist)
    }

    fn lyrics(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Lyrics)
    }

    fn mixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Mixer)
    }

    fn mood(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Mood)
    }

    fn movement(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Movement)
    }

    fn movement_count(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MovementCount)
    }

    fn movement_number(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MovementNumber)
    }

    fn musicbrainz_artist_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicBrainzArtistId)
    }

    fn musicbrainz_original_artist_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicBrainzOriginalArtistId)
    }

    fn musicbrainz_original_release_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicBrainzOriginalReleaseId)
    }

    fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicBrainzRecordingId)
    }

    fn musicbrainz_track_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicBrainzTrackId)
    }

    fn musicbrainz_trm_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicBrainzTrmId)
    }

    fn musicbrainz_work_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicBrainzWorkId)
    }

    fn musicip_fingerprint(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicIpFingerprint)
    }

    fn musicip_puid(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::MusicIpPuid)
    }

    fn original_album(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::OriginalAlbum)
    }

    fn original_artist(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::OriginalArtist)
    }

    fn original_filename(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::OriginalFilename)
    }

    fn original_release_date(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::OriginalReleaseDate)
    }

    fn original_release_year(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::OriginalReleaseYear)
    }

    fn performers(&self) -> Option<Vec<InvolvedPerson<'_>>> {
        self.tags().iter().find_map(|tag| tag.performers())
    }

    fn producer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Producer)
    }

    fn rating(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::Rating)
    }

    fn remixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Remixer)
    }

    fn replay_gain_album_gain(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ReplayGainAlbumGain)
    }

    fn replay_gain_album_peak(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ReplayGainAlbumPeak)
    }

    fn replay_gain_album_range(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ReplayGainAlbumRange)
    }

    fn replay_gain_reference_loudness(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ReplayGainReferenceLoudness)
    }

    fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ReplayGainTrackGain)
    }

    fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ReplayGainTrackPeak)
    }

    fn replay_gain_track_range(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ReplayGainTrackRange)
    }

    fn track_number(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::TrackNumber)
    }

    fn track_title(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::TrackTitle)
    }

    fn track_title_sort_order(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::TrackTitleSortOrder)
    }

    fn artist_website(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::ArtistWebsite)
    }

    fn work_title(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(&TagKey::WorkTitle)
    }

    fn writer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(&TagKey::Writer)
    }

    fn track_length(&self) -> Option<chrono::TimeDelta> {
        self.analysis_results
            .as_ref()
            .and_then(|results| results.track_length.as_ref())
            .and_then(|track_length| track_length.as_ref().ok().copied())
    }

    fn track_path(&self) -> Option<&Path> {
        self.path.as_path().into()
    }

    fn analyzed_metadata(&self) -> impl AnalyzedTrackMetadata {
        TaggedFileAnalyzedMetadata(self.analysis_results.as_ref())
    }
}

/// Analyzed metadata for tagged file metadata.
struct TaggedFileAnalyzedMetadata<'a>(Option<&'a CompoundAnalyzerResult>);

impl AnalyzedTrackMetadata for TaggedFileAnalyzedMetadata<'_> {
    fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>> {
        self.0
            .and_then(|result| result.chromaprint_fingerprint.as_ref())
            .and_then(|res| res.as_ref().ok())
            .map(|fp| Cow::from(fp.fingerprint_string()))
    }

    fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>> {
        self.0
            .and_then(|result| result.ebur128.as_ref())
            .and_then(|res| res.as_ref().ok())
            .map(|ebur128| Cow::from(ebur128.replaygain_track_gain_string()))
    }

    fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>> {
        self.0
            .and_then(|result| result.ebur128.as_ref())
            .and_then(|res| res.as_ref().ok())
            .map(|ebur128| Cow::from(ebur128.replaygain_track_peak_string()))
    }

    fn replay_gain_track_range(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn bpm(&self) -> Option<Cow<'_, str>> {
        self.0
            .and_then(|result| result.soundtouch_bpm.as_ref())
            .and_then(|res| res.as_ref().ok())
            .map(|soundtouch_bpm| Cow::from(soundtouch_bpm.bpm_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaggedFileCollection;
    use musicbrainz_rs_nova::entity::release::{
        Release as MusicBrainzRelease, Track as MusicBrainzTrack,
    };

    const MUSICBRAINZ_RELEASE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/musicbrainz/release.json"
    ));

    #[cfg(feature = "id3")]
    #[test]
    fn test_assign_tags_from_track_id3() {
        use crate::tag::id3::ID3v2Tag;

        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let track: &MusicBrainzTrack =
            &release.media.as_ref().unwrap()[0].tracks.as_ref().unwrap()[0];

        let mut tagged_file = TaggedFile::new(vec![Box::new(ID3v2Tag::default())]);
        assert!(tagged_file.track_title().is_none());
        assert!(tagged_file.track_artist().is_none());
        assert!(tagged_file.track_number().is_none());
        assert!(tagged_file.musicbrainz_recording_id().is_none());

        tagged_file.assign_tags_from_track(track);

        assert!(tagged_file.track_title().is_some());
        assert!(tagged_file.track_artist().is_some());
        assert!(tagged_file.track_number().is_some());
        assert!(tagged_file.musicbrainz_recording_id().is_some());
    }

    #[cfg(feature = "id3")]
    #[test]
    fn test_assign_tags_from_release_id3() {
        use crate::tag::id3::ID3v2Tag;

        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();

        let tagged_file = TaggedFile::new(vec![Box::new(ID3v2Tag::default())]);
        let tagged_file_collection = TaggedFileCollection::new(vec![tagged_file]);
        assert!(tagged_file_collection.release_title().is_none());
        assert!(tagged_file_collection.release_artist().is_none());
        assert!(tagged_file_collection.release_country().is_none());
        assert!(tagged_file_collection.musicbrainz_release_id().is_none());

        let mut tagged_file = tagged_file_collection.into_iter().next().unwrap();

        tagged_file.assign_tags_from_release(&release);

        let tagged_file_collection = TaggedFileCollection::new(vec![tagged_file]);
        assert!(tagged_file_collection.release_title().is_some());
        assert!(tagged_file_collection.release_artist().is_some());
        assert!(tagged_file_collection.release_country().is_some());
        assert!(tagged_file_collection.musicbrainz_release_id().is_some());
    }

    #[cfg(feature = "flac")]
    #[test]
    fn test_assign_tags_from_track_flac() {
        use crate::tag::flac::FlacTag;

        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let track: &MusicBrainzTrack =
            &release.media.as_ref().unwrap()[0].tracks.as_ref().unwrap()[0];

        let mut tagged_file = TaggedFile::new(vec![Box::new(FlacTag::new())]);
        assert!(tagged_file.track_title().is_none());
        assert!(tagged_file.track_artist().is_none());
        assert!(tagged_file.track_number().is_none());
        assert!(tagged_file.musicbrainz_recording_id().is_none());

        tagged_file.assign_tags_from_track(track);

        assert!(tagged_file.track_title().is_some());
        assert!(tagged_file.track_artist().is_some());
        assert!(tagged_file.track_number().is_some());
        assert!(tagged_file.musicbrainz_recording_id().is_some());
    }

    #[cfg(feature = "flac")]
    #[test]
    fn test_assign_tags_from_release_flac() {
        use crate::tag::flac::FlacTag;

        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();

        let tagged_file = TaggedFile::new(vec![Box::new(FlacTag::new())]);
        let tagged_file_collection = TaggedFileCollection::new(vec![tagged_file]);
        assert!(tagged_file_collection.release_title().is_none());
        assert!(tagged_file_collection.release_artist().is_none());
        assert!(tagged_file_collection.release_country().is_none());
        assert!(tagged_file_collection.musicbrainz_release_id().is_none());

        let mut tagged_file = tagged_file_collection.into_iter().next().unwrap();

        tagged_file.assign_tags_from_release(&release);

        let tagged_file_collection = TaggedFileCollection::new(vec![tagged_file]);
        assert!(tagged_file_collection.release_title().is_some());
        assert!(tagged_file_collection.release_artist().is_some());
        assert!(tagged_file_collection.release_country().is_some());
        assert!(tagged_file_collection.musicbrainz_release_id().is_some());
    }
}
