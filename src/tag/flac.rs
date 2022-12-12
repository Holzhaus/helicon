// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Support for FLAC tags.

#![cfg(feature = "flac")]

use crate::tag::{Tag, TagKey, TagType};
use std::path::Path;

/// FLAC tag.
pub struct FlacTag {
    /// The underlying tag data.
    data: metaflac::Tag,
}

impl FlacTag {
    /// Read the FLAC tag from the path
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        let data = metaflac::Tag::read_from_path(path)?;
        Ok(FlacTag { data })
    }

    /// Get the vorbis key name for a tag key.
    fn tag_key_to_frame(key: &TagKey) -> Option<&'static str> {
        #[allow(clippy::match_same_arms)]
        match key {
            TagKey::AcoustId => "ACOUSTID_ID".into(),
            TagKey::AcoustIdFingerprint => "ACOUSTID_FINGERPRINT".into(),
            TagKey::Album => "ALBUM".into(),
            TagKey::AlbumArtist => "ALBUMARTIST".into(),
            TagKey::AlbumArtistSortOrder => "ALBUMARTISTSORT".into(),
            TagKey::AlbumSortOrder => "ALBUMSORT".into(),
            TagKey::Arranger => "ARRANGER".into(),
            TagKey::Artist => "ARTIST".into(),
            TagKey::ArtistSortOrder => "ARTISTSORT".into(),
            TagKey::Artists => "ARTISTS".into(),
            TagKey::Asin => "ASIN".into(),
            TagKey::Barcode => "BARCODE".into(),
            TagKey::Bpm => "BPM".into(),
            TagKey::CatalogNumber => "CATALOGNUMBER".into(),
            TagKey::Comment => "COMMENT".into(),
            TagKey::Compilation => "COMPILATION".into(),
            TagKey::Composer => "COMPOSER".into(),
            TagKey::ComposerSortOrder => "COMPOSERSORT".into(),
            TagKey::Conductor => "CONDUCTOR".into(),
            TagKey::Copyright => "COPYRIGHT".into(),
            TagKey::Director => "DIRECTOR".into(),
            TagKey::DiscNumber => "DISCNUMBER".into(),
            TagKey::DiscSubtitle => "DISCSUBTITLE".into(),
            TagKey::EncodedBy => "ENCODEDBY".into(),
            TagKey::EncoderSettings => "ENCODERSETTINGS".into(),
            TagKey::Engineer => "ENGINEER".into(),
            TagKey::GaplessPlayback => None,
            TagKey::Genre => "GENRE".into(),
            TagKey::Grouping => "GROUPING".into(),
            TagKey::InitialKey => "KEY".into(),
            TagKey::Isrc => "ISRC".into(),
            TagKey::Language => "LANGUAGE".into(),
            TagKey::License => "LICENSE".into(),
            TagKey::Lyricist => "LYRICIST".into(),
            TagKey::Lyrics => "LYRICS".into(),
            TagKey::Media => "MEDIA".into(),
            TagKey::DjMixer => "DJMIXER".into(),
            TagKey::Mixer => "MIXER".into(),
            TagKey::Mood => "MOOD".into(),
            TagKey::Movement => "MOVEMENTNAME".into(),
            TagKey::MovementCount => "MOVEMENTTOTAL".into(),
            TagKey::MovementNumber => "MOVEMENT".into(),
            TagKey::MusicBrainzArtistId => "MUSICBRAINZ_ARTISTID".into(),
            TagKey::MusicBrainzDiscId => "MUSICBRAINZ_DISCID".into(),
            TagKey::MusicBrainzOriginalArtistId => "MUSICBRAINZ_ORIGINALARTISTID".into(),
            TagKey::MusicBrainzOriginalReleaseId => "MUSICBRAINZ_ORIGINALALBUMID".into(),
            TagKey::MusicBrainzRecordingId => "MUSICBRAINZ_TRACKID".into(),
            TagKey::MusicBrainzReleaseArtistId => "MUSICBRAINZ_ALBUMARTISTID".into(),
            TagKey::MusicBrainzReleaseGroupId => "MUSICBRAINZ_RELEASEGROUPID".into(),
            TagKey::MusicBrainzReleaseId => "MUSICBRAINZ_ALBUMID".into(),
            TagKey::MusicBrainzTrackId => "MUSICBRAINZ_RELEASETRACKID".into(),
            TagKey::MusicBrainzTrmId => "MUSICBRAINZ_TRMID".into(),
            TagKey::MusicBrainzWorkId => "MUSICBRAINZ_WORKID".into(),
            TagKey::MusicIpFingerprint => None, // TODO: Add mapping to "FINGERPRINT=MusicMagic Fingerprint {fingerprint}"
            TagKey::MusicIpPuid => "MUSICIP_PUID".into(),
            TagKey::OriginalAlbum => None,
            TagKey::OriginalArtist => None,
            TagKey::OriginalFilename => "ORIGINALFILENAME".into(),
            TagKey::OriginalReleaseDate => "ORIGINALDATE".into(),
            TagKey::OriginalReleaseYear => "ORIGINALYEAR".into(),
            TagKey::Performer => "PERFORMER={artist} (instrument)".into(),
            TagKey::Podcast => None,
            TagKey::PodcastUrl => None,
            TagKey::Producer => "PRODUCER".into(),
            TagKey::Rating => None, // TODO: Add mapping to "RATING:user@email"
            TagKey::RecordLabel => "LABEL".into(),
            TagKey::ReleaseCountry => "RELEASECOUNTRY".into(),
            TagKey::ReleaseDate => "DATE".into(),
            TagKey::ReleaseYear => None,
            TagKey::ReleaseStatus => "RELEASESTATUS".into(),
            TagKey::ReleaseType => "RELEASETYPE".into(),
            TagKey::Remixer => "REMIXER".into(),
            TagKey::ReplayGainAlbumGain => "REPLAYGAIN_ALBUM_GAIN".into(),
            TagKey::ReplayGainAlbumPeak => "REPLAYGAIN_ALBUM_PEAK".into(),
            TagKey::ReplayGainAlbumRange => "REPLAYGAIN_ALBUM_RANGE".into(),
            TagKey::ReplayGainReferenceLoudness => "REPLAYGAIN_REFERENCE_LOUDNESS".into(),
            TagKey::ReplayGainTrackGain => "REPLAYGAIN_TRACK_GAIN".into(),
            TagKey::ReplayGainTrackPeak => "REPLAYGAIN_TRACK_PEAK".into(),
            TagKey::ReplayGainTrackRange => "REPLAYGAIN_TRACK_RANGE".into(),
            TagKey::Script => "SCRIPT".into(),
            TagKey::ShowName => None,
            TagKey::ShowNameSortOrder => None,
            TagKey::ShowMovement => "SHOWMOVEMENT".into(),
            TagKey::Subtitle => "SUBTITLE".into(),
            TagKey::TotalDiscs => "DISCTOTAL and TOTALDISCS".into(),
            TagKey::TotalTracks => "TRACKTOTAL and TOTALTRACKS".into(),
            TagKey::TrackNumber => "TRACKNUMBER".into(),
            TagKey::TrackTitle => "TITLE".into(),
            TagKey::TrackTitleSortOrder => "TITLESORT".into(),
            TagKey::ArtistWebsite => "WEBSITE".into(),
            TagKey::WorkTitle => "WORK".into(),
            TagKey::Writer => "WRITER".into(),
        }
    }
}

impl Tag for FlacTag {
    fn tag_type(&self) -> TagType {
        TagType::Flac
    }

    fn get(&self, key: &TagKey) -> Option<&str> {
        Self::tag_key_to_frame(key)
            .and_then(|key| self.data.get_vorbis(key))
            .and_then(|mut iterator| iterator.next())
    }
}
