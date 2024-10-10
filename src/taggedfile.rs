// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! The [`TaggedFile`] struct represents a file that contains tags.

use crate::analyzer::CompoundAnalyzerResult;
use crate::release::ReleaseLike;
use crate::tag::{read_tags_from_path, Tag, TagKey};
use crate::track::TrackLike;
use std::borrow::Cow;
use std::fmt;
use std::path::{Path, PathBuf};

/// A tagged file that contains zero or more tags.
pub struct TaggedFile {
    /// Path of the file.
    path: PathBuf,
    /// Tags that are present in the file.
    content: Vec<Box<dyn Tag>>,
    /// Analysis results.
    analysis_results: Option<CompoundAnalyzerResult>,
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
    #[cfg(test)]
    #[must_use]
    pub fn new(content: Vec<Box<dyn Tag>>) -> Self {
        TaggedFile {
            path: PathBuf::new(),
            content,
            analysis_results: None,
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
    pub fn tag_values(&self, key: TagKey) -> impl Iterator<Item = &str> {
        self.tags().iter().filter_map(move |tag| tag.get(key))
    }

    /// Yields all values for the given [`TagKey`].
    #[expect(clippy::needless_pass_by_value)]
    pub fn set_tag_value(&mut self, key: TagKey, value: Option<Cow<'_, str>>) {
        self.content
            .iter_mut()
            .for_each(|tag| tag.set_or_clear(key, value.clone()));
    }

    /// Yields all values for the given [`TagKey`].
    pub fn set_tag_values(&mut self, key: TagKey, values: &[Cow<'_, str>]) {
        self.content
            .iter_mut()
            .for_each(|tag| tag.set_multiple(key, values));
    }

    /// Returns the first value for the given [`TagKey`].
    #[must_use]
    pub fn first_tag_value(&self, key: TagKey) -> Option<&str> {
        self.tag_values(key).next()
    }

    /// Assign metadata from a `ReleaseLike` struct (e.g. a MusicBrainz release).
    pub fn assign_tags_from_release(&mut self, release: &impl ReleaseLike) {
        self.set_tag_value(TagKey::Album, release.release_title());
        self.set_tag_value(TagKey::AlbumArtist, release.release_artist());
        self.set_tag_value(
            TagKey::AlbumArtistSortOrder,
            release.release_artist_sort_order(),
        );
        self.set_tag_value(TagKey::AlbumSortOrder, release.release_sort_order());
        self.set_tag_value(TagKey::Asin, release.asin());
        self.set_tag_value(TagKey::Barcode, release.barcode());
        self.set_tag_value(TagKey::CatalogNumber, release.catalog_number());
        self.set_tag_value(TagKey::Compilation, release.compilation());
        self.set_tag_value(TagKey::Grouping, release.grouping());
        self.set_tag_value(
            TagKey::MusicBrainzReleaseArtistId,
            release.musicbrainz_release_artist_id(),
        );
        self.set_tag_value(
            TagKey::MusicBrainzReleaseGroupId,
            release.musicbrainz_release_group_id(),
        );
        self.set_tag_value(
            TagKey::MusicBrainzReleaseId,
            release.musicbrainz_release_id(),
        );
        self.set_tag_value(TagKey::RecordLabel, release.record_label());
        self.set_tag_value(TagKey::ReleaseCountry, release.release_country());
        self.set_tag_value(TagKey::ReleaseDate, release.release_date());
        self.set_tag_value(TagKey::ReleaseYear, release.release_year());
        self.set_tag_value(TagKey::ReleaseStatus, release.release_status());
        self.set_tag_value(TagKey::ReleaseType, release.release_type());
        self.set_tag_value(TagKey::Script, release.script());
        self.set_tag_value(TagKey::TotalDiscs, release.total_discs());
    }

    /// Assign metadata from another `TrackLike` struct (e.g. a MusicBrainz track).
    pub fn assign_tags_from_track(&mut self, track: &impl TrackLike) {
        self.set_tag_value(TagKey::AcoustId, track.acoustid());
        self.set_tag_value(TagKey::AcoustIdFingerprint, track.acoustid_fingerprint());
        self.set_tag_values(
            TagKey::Arranger,
            track.arranger().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(TagKey::Artist, track.track_artist());
        self.set_tag_value(TagKey::ArtistSortOrder, track.track_artist_sort_order());
        self.set_tag_value(TagKey::Artists, track.track_artist());
        self.set_tag_value(TagKey::Bpm, track.bpm());
        self.set_tag_value(TagKey::Comment, track.comment());
        self.set_tag_values(
            TagKey::Composer,
            track.composer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(TagKey::ComposerSortOrder, track.composer_sort_order());
        self.set_tag_values(
            TagKey::Conductor,
            track.conductor().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(TagKey::Copyright, track.copyright());
        self.set_tag_values(
            TagKey::Director,
            track.director().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_values(
            TagKey::DjMixer,
            track.dj_mixer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(TagKey::EncodedBy, track.encoded_by());
        self.set_tag_value(TagKey::EncoderSettings, track.encoder_settings());
        self.set_tag_values(
            TagKey::Engineer,
            track.engineer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_values(TagKey::Genre, track.genre().collect::<Vec<_>>().as_slice());
        self.set_tag_value(TagKey::InitialKey, track.initial_key());
        self.set_tag_values(TagKey::Isrc, track.isrc().collect::<Vec<_>>().as_slice());
        self.set_tag_value(TagKey::Language, track.language());
        self.set_tag_value(TagKey::License, track.license());
        self.set_tag_values(
            TagKey::Lyricist,
            track.lyricist().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(TagKey::Lyrics, track.lyrics());
        self.set_tag_values(TagKey::Mixer, track.mixer().collect::<Vec<_>>().as_slice());
        self.set_tag_value(TagKey::Mood, track.mood());
        self.set_tag_value(TagKey::Movement, track.movement());
        self.set_tag_value(TagKey::MovementCount, track.movement_count());
        self.set_tag_value(TagKey::MovementNumber, track.movement_number());
        self.set_tag_value(TagKey::MusicBrainzArtistId, track.musicbrainz_artist_id());
        self.set_tag_value(
            TagKey::MusicBrainzOriginalArtistId,
            track.musicbrainz_original_artist_id(),
        );
        self.set_tag_value(
            TagKey::MusicBrainzOriginalReleaseId,
            track.musicbrainz_original_release_id(),
        );
        self.set_tag_value(
            TagKey::MusicBrainzRecordingId,
            track.musicbrainz_recording_id(),
        );
        self.set_tag_value(TagKey::MusicBrainzTrackId, track.musicbrainz_track_id());
        self.set_tag_value(TagKey::MusicBrainzTrmId, track.musicbrainz_trm_id());
        self.set_tag_value(TagKey::MusicBrainzWorkId, track.musicbrainz_work_id());
        self.set_tag_value(TagKey::MusicIpFingerprint, track.musicip_fingerprint());
        self.set_tag_value(TagKey::MusicIpPuid, track.musicip_puid());
        self.set_tag_value(TagKey::OriginalAlbum, track.original_album());
        self.set_tag_value(TagKey::OriginalArtist, track.original_artist());
        self.set_tag_value(TagKey::OriginalFilename, track.original_filename());
        self.set_tag_value(TagKey::OriginalReleaseDate, track.original_release_date());
        self.set_tag_value(TagKey::OriginalReleaseYear, track.original_release_year());
        self.set_tag_values(
            TagKey::Performer,
            track.performer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_values(
            TagKey::Producer,
            track.producer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(TagKey::Rating, track.rating());
        self.set_tag_values(
            TagKey::Remixer,
            track.remixer().collect::<Vec<_>>().as_slice(),
        );
        self.set_tag_value(TagKey::ReplayGainAlbumGain, track.replay_gain_album_gain());
        self.set_tag_value(TagKey::ReplayGainAlbumPeak, track.replay_gain_album_peak());
        self.set_tag_value(
            TagKey::ReplayGainAlbumRange,
            track.replay_gain_album_range(),
        );
        self.set_tag_value(
            TagKey::ReplayGainReferenceLoudness,
            track.replay_gain_reference_loudness(),
        );
        self.set_tag_value(TagKey::ReplayGainTrackGain, track.replay_gain_track_gain());
        self.set_tag_value(TagKey::ReplayGainTrackPeak, track.replay_gain_track_peak());
        self.set_tag_value(
            TagKey::ReplayGainTrackRange,
            track.replay_gain_track_range(),
        );
        self.set_tag_value(TagKey::TrackNumber, track.track_number());
        self.set_tag_value(TagKey::TrackTitle, track.track_title());
        self.set_tag_value(TagKey::TrackTitleSortOrder, track.track_title_sort_order());
        self.set_tag_value(TagKey::ArtistWebsite, track.artist_website());
        self.set_tag_value(TagKey::WorkTitle, track.work_title());
        self.set_tag_values(
            TagKey::Writer,
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

impl TrackLike for TaggedFile {
    fn acoustid(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::AcoustId).map(Cow::from)
    }

    fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::AcoustIdFingerprint)
            .map(Cow::from)
    }

    fn arranger(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Arranger).map(Cow::from)
    }

    fn track_artist(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Artist)
            .or_else(|| self.first_tag_value(TagKey::Artists))
            .or_else(|| self.first_tag_value(TagKey::AlbumArtist))
            .map(Cow::from)
    }

    fn track_artist_sort_order(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ArtistSortOrder).map(Cow::from)
    }

    fn bpm(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Bpm).map(Cow::from)
    }

    fn comment(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Comment).map(Cow::from)
    }

    fn composer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Composer).map(Cow::from)
    }

    fn composer_sort_order(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ComposerSortOrder)
            .map(Cow::from)
    }

    fn conductor(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Conductor).map(Cow::from)
    }

    fn copyright(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Copyright).map(Cow::from)
    }

    fn director(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Director).map(Cow::from)
    }

    fn dj_mixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::DjMixer).map(Cow::from)
    }

    fn encoded_by(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::EncodedBy).map(Cow::from)
    }

    fn encoder_settings(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::EncoderSettings).map(Cow::from)
    }

    fn engineer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Engineer).map(Cow::from)
    }

    fn genre(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Genre).map(Cow::from)
    }

    fn initial_key(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::InitialKey).map(Cow::from)
    }

    fn isrc(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Isrc).map(Cow::from)
    }

    fn language(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Language).map(Cow::from)
    }

    fn license(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::License).map(Cow::from)
    }

    fn lyricist(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Lyricist).map(Cow::from)
    }

    fn lyrics(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Lyrics).map(Cow::from)
    }

    fn mixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Mixer).map(Cow::from)
    }

    fn mood(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Mood).map(Cow::from)
    }

    fn movement(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Movement).map(Cow::from)
    }

    fn movement_count(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MovementCount).map(Cow::from)
    }

    fn movement_number(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MovementNumber).map(Cow::from)
    }

    fn musicbrainz_artist_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicBrainzArtistId)
            .map(Cow::from)
    }

    fn musicbrainz_original_artist_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicBrainzOriginalArtistId)
            .map(Cow::from)
    }

    fn musicbrainz_original_release_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicBrainzOriginalReleaseId)
            .map(Cow::from)
    }

    fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicBrainzRecordingId)
            .map(Cow::from)
    }

    fn musicbrainz_track_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicBrainzTrackId)
            .map(Cow::from)
    }

    fn musicbrainz_trm_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicBrainzTrmId)
            .map(Cow::from)
    }

    fn musicbrainz_work_id(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicBrainzWorkId)
            .map(Cow::from)
    }

    fn musicip_fingerprint(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicIpFingerprint)
            .map(Cow::from)
    }

    fn musicip_puid(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::MusicIpPuid).map(Cow::from)
    }

    fn original_album(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::OriginalAlbum).map(Cow::from)
    }

    fn original_artist(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::OriginalArtist).map(Cow::from)
    }

    fn original_filename(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::OriginalFilename)
            .map(Cow::from)
    }

    fn original_release_date(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::OriginalReleaseDate)
            .map(Cow::from)
    }

    fn original_release_year(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::OriginalReleaseYear)
            .map(Cow::from)
    }

    fn performer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Performer).map(Cow::from)
    }

    fn producer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Producer).map(Cow::from)
    }

    fn rating(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::Rating).map(Cow::from)
    }

    fn remixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Remixer).map(Cow::from)
    }

    fn replay_gain_album_gain(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ReplayGainAlbumGain)
            .map(Cow::from)
    }

    fn replay_gain_album_peak(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ReplayGainAlbumPeak)
            .map(Cow::from)
    }

    fn replay_gain_album_range(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ReplayGainAlbumRange)
            .map(Cow::from)
    }

    fn replay_gain_reference_loudness(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ReplayGainReferenceLoudness)
            .map(Cow::from)
    }

    fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ReplayGainTrackGain)
            .map(Cow::from)
    }

    fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ReplayGainTrackPeak)
            .map(Cow::from)
    }

    fn replay_gain_track_range(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ReplayGainTrackRange)
            .map(Cow::from)
    }

    fn track_number(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::TrackNumber).map(Cow::from)
    }

    fn track_title(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::TrackTitle).map(Cow::from)
    }

    fn track_title_sort_order(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::TrackTitleSortOrder)
            .map(Cow::from)
    }

    fn artist_website(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::ArtistWebsite).map(Cow::from)
    }

    fn work_title(&self) -> Option<Cow<'_, str>> {
        self.first_tag_value(TagKey::WorkTitle).map(Cow::from)
    }

    fn writer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.tag_values(TagKey::Writer).map(Cow::from)
    }

    fn track_length(&self) -> Option<chrono::TimeDelta> {
        self.analysis_results
            .as_ref()
            .and_then(|results| results.track_length.as_ref())
            .and_then(|track_length| track_length.as_ref().ok().copied())
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

        let mut tagged_file = TaggedFile::new(vec![Box::new(ID3v2Tag::new())]);
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

        let tagged_file = TaggedFile::new(vec![Box::new(ID3v2Tag::new())]);
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
