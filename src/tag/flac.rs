// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Support for FLAC tags.

use crate::tag::{Tag, TagKey, TagType};
use std::borrow::Cow;
use std::path::Path;

/// FLAC tag.
pub struct FlacTag {
    /// The underlying tag data.
    data: metaflac::Tag,
}

impl FlacTag {
    #[cfg(test)]
    pub fn new() -> Self {
        FlacTag {
            data: metaflac::Tag::new(),
        }
    }

    /// Read the FLAC tag from the path
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        let data = metaflac::Tag::read_from_path(path)?;
        Ok(FlacTag { data })
    }

    /// Get the vorbis key name for a tag key.
    fn tag_key_to_frame(key: TagKey) -> Option<&'static str> {
        #[expect(clippy::match_same_arms)]
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

    fn get(&self, key: TagKey) -> Option<&str> {
        Self::tag_key_to_frame(key)
            .and_then(|key| self.data.get_vorbis(key))
            .and_then(|mut iterator| iterator.next())
    }

    fn set(&mut self, key: TagKey, value: Cow<'_, str>) {
        if let Some(frame) = Self::tag_key_to_frame(key) {
            self.data.set_vorbis(frame, vec![value]);
        }
    }

    fn clear(&mut self, key: TagKey) {
        if let Some(frame) = Self::tag_key_to_frame(key) {
            self.data.remove_vorbis(frame);
        }
    }

    fn write(&mut self, path: &Path) -> crate::Result<()> {
        self.data.write_to_path(path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag::{Tag, TagKey};
    use paste::paste;

    #[test]
    fn test_tag_type() {
        let tag = FlacTag::new();
        assert_eq!(tag.tag_type(), TagType::Flac);
    }

    macro_rules! add_tests {
        ($tagkey:expr, $fnsuffix:ident) => {
            paste! {
                #[test]
                fn [<test_get_set_ $fnsuffix>]() {
                    let mut tag = FlacTag::new();
                    assert!(tag.get($tagkey).is_none());

                    tag.set($tagkey, Cow::from("Example Value"));
                    assert_eq!(tag.get($tagkey), Some("Example Value"));
                }

                #[test]
                fn [<test_clear_ $fnsuffix>]() {
                    let mut tag = FlacTag::new();
                    assert!(tag.get($tagkey).is_none());

                    tag.set($tagkey, Cow::from("Example Value"));
                    assert!(tag.get($tagkey).is_some());

                    tag.clear($tagkey);
                    assert!(tag.get($tagkey).is_none());
                }

                #[test]
                fn [<test_set_or_clear_ $fnsuffix>]() {
                    let mut tag = FlacTag::new();
                    assert!(tag.get($tagkey).is_none());

                    tag.set_or_clear($tagkey, Some(Cow::from("Example Value")));
                    assert!(tag.get($tagkey).is_some());

                    tag.set_or_clear($tagkey, Some(Cow::from("Other Value")));
                    assert!(tag.get($tagkey).is_some());

                    tag.set_or_clear($tagkey, None);
                    assert!(tag.get($tagkey).is_none());
                }
            }
        };
    }

    add_tests!(TagKey::AcoustId, acoustid);
    add_tests!(TagKey::AcoustIdFingerprint, acoustidfingerprint);
    add_tests!(TagKey::Album, album);
    add_tests!(TagKey::AlbumArtist, albumartist);
    add_tests!(TagKey::AlbumArtistSortOrder, albumartistsortorder);
    add_tests!(TagKey::AlbumSortOrder, albumsortorder);
    add_tests!(TagKey::Arranger, arranger);
    add_tests!(TagKey::Artist, artist);
    add_tests!(TagKey::ArtistSortOrder, artistsortorder);
    add_tests!(TagKey::Artists, artists);
    add_tests!(TagKey::Asin, asin);
    add_tests!(TagKey::Barcode, barcode);
    add_tests!(TagKey::Bpm, bpm);
    add_tests!(TagKey::CatalogNumber, catalognumber);
    add_tests!(TagKey::Comment, comment);
    add_tests!(TagKey::Compilation, compilation);
    add_tests!(TagKey::Composer, composer);
    add_tests!(TagKey::ComposerSortOrder, composersortorder);
    add_tests!(TagKey::Conductor, conductor);
    add_tests!(TagKey::Copyright, copyright);
    add_tests!(TagKey::Director, director);
    add_tests!(TagKey::DiscNumber, discnumber);
    add_tests!(TagKey::DiscSubtitle, discsubtitle);
    add_tests!(TagKey::EncodedBy, encodedby);
    add_tests!(TagKey::EncoderSettings, encodersettings);
    add_tests!(TagKey::Engineer, engineer);
    //add_tests!(TagKey::GaplessPlayback, gaplessplayback);
    add_tests!(TagKey::Genre, genre);
    add_tests!(TagKey::Grouping, grouping);
    add_tests!(TagKey::InitialKey, initialkey);
    add_tests!(TagKey::Isrc, isrc);
    add_tests!(TagKey::Language, language);
    add_tests!(TagKey::License, license);
    add_tests!(TagKey::Lyricist, lyricist);
    add_tests!(TagKey::Lyrics, lyrics);
    add_tests!(TagKey::Media, media);
    add_tests!(TagKey::DjMixer, djmixer);
    add_tests!(TagKey::Mixer, mixer);
    add_tests!(TagKey::Mood, mood);
    add_tests!(TagKey::Movement, movement);
    add_tests!(TagKey::MovementCount, movementcount);
    add_tests!(TagKey::MovementNumber, movementnumber);
    add_tests!(TagKey::MusicBrainzArtistId, musicbrainzartistid);
    add_tests!(TagKey::MusicBrainzDiscId, musicbrainzdiscid);
    add_tests!(
        TagKey::MusicBrainzOriginalArtistId,
        musicbrainzoriginalartistid
    );
    add_tests!(
        TagKey::MusicBrainzOriginalReleaseId,
        musicbrainzoriginalreleaseid
    );
    add_tests!(TagKey::MusicBrainzRecordingId, musicbrainzrecordingid);
    add_tests!(
        TagKey::MusicBrainzReleaseArtistId,
        musicbrainzreleaseartistid
    );
    add_tests!(TagKey::MusicBrainzReleaseGroupId, musicbrainzreleasegroupid);
    add_tests!(TagKey::MusicBrainzReleaseId, musicbrainzreleaseid);
    add_tests!(TagKey::MusicBrainzTrackId, musicbrainztrackid);
    add_tests!(TagKey::MusicBrainzTrmId, musicbrainztrmid);
    add_tests!(TagKey::MusicBrainzWorkId, musicbrainzworkid);
    //add_tests!(TagKey::MusicIpFingerprint, musicipfingerprint);
    add_tests!(TagKey::MusicIpPuid, musicippuid);
    //add_tests!(TagKey::OriginalAlbum, originalalbum);
    //add_tests!(TagKey::OriginalArtist, originalartist);
    add_tests!(TagKey::OriginalFilename, originalfilename);
    add_tests!(TagKey::OriginalReleaseDate, originalreleasedate);
    add_tests!(TagKey::OriginalReleaseYear, originalreleaseyear);
    add_tests!(TagKey::Performer, performer);
    //add_tests!(TagKey::Podcast, podcast);
    //add_tests!(TagKey::PodcastUrl, podcasturl);
    add_tests!(TagKey::Producer, producer);
    //add_tests!(TagKey::Rating, rating);
    add_tests!(TagKey::RecordLabel, recordlabel);
    add_tests!(TagKey::ReleaseCountry, releasecountry);
    add_tests!(TagKey::ReleaseDate, releasedate);
    //add_tests!(TagKey::ReleaseYear, releaseyear);
    add_tests!(TagKey::ReleaseStatus, releasestatus);
    add_tests!(TagKey::ReleaseType, releasetype);
    add_tests!(TagKey::Remixer, remixer);
    add_tests!(TagKey::ReplayGainAlbumGain, replaygainalbumgain);
    add_tests!(TagKey::ReplayGainAlbumPeak, replaygainalbumpeak);
    add_tests!(TagKey::ReplayGainAlbumRange, replaygainalbumrange);
    add_tests!(
        TagKey::ReplayGainReferenceLoudness,
        replaygainreferenceloudness
    );
    add_tests!(TagKey::ReplayGainTrackGain, replaygaintrackgain);
    add_tests!(TagKey::ReplayGainTrackPeak, replaygaintrackpeak);
    add_tests!(TagKey::ReplayGainTrackRange, replaygaintrackrange);
    add_tests!(TagKey::Script, script);
    //add_tests!(TagKey::ShowName, showname);
    //add_tests!(TagKey::ShowNameSortOrder, shownamesortorder);
    add_tests!(TagKey::ShowMovement, showmovement);
    add_tests!(TagKey::Subtitle, subtitle);
    add_tests!(TagKey::TotalDiscs, totaldiscs);
    add_tests!(TagKey::TotalTracks, totaltracks);
    add_tests!(TagKey::TrackNumber, tracknumber);
    add_tests!(TagKey::TrackTitle, tracktitle);
    add_tests!(TagKey::TrackTitleSortOrder, tracktitlesortorder);
    add_tests!(TagKey::ArtistWebsite, artistwebsite);
    add_tests!(TagKey::WorkTitle, worktitle);
    add_tests!(TagKey::Writer, writer);
}
