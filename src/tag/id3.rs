// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Support for ID3 tags.

use crate::tag::{Tag, TagKey, TagType};
use id3::{
    frame::{ExtendedText, UniqueFileIdentifier},
    TagLike,
};
use std::borrow::{Borrow, Cow};
use std::iter;
use std::path::Path;

/// ID3 frame ID.
#[derive(Debug)]
enum FrameId<'a> {
    /// Text frame.
    Text(&'a str),
    /// Extended Text frame (`TXXX`).
    ExtendedText(&'a str),
    /// Text Frame with multiple values.
    MultiValuedText(&'a str, &'a str),
    /// Unique File Identifier frame (`UFID`).
    UniqueFileIdentifier(&'a str),
}

/// ID3 tag (version 2).
#[derive(Debug)]
pub struct ID3v2Tag {
    /// The underlying tag data.
    data: id3::Tag,
}

impl ID3v2Tag {
    #[cfg(test)]
    pub fn new() -> Self {
        ID3v2Tag {
            data: id3::Tag::new(),
        }
    }

    #[cfg(test)]
    pub fn with_version(version: id3::Version) -> Self {
        ID3v2Tag {
            data: id3::Tag::with_version(version),
        }
    }

    /// Read the ID3 tag from the path
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        let data = id3::Tag::read_from_path(path)?;
        Ok(ID3v2Tag { data })
    }

    /// Get the ID3 frame for a tag key.
    fn tag_key_to_frame(&self, key: TagKey) -> Option<FrameId<'static>> {
        #[expect(clippy::match_same_arms)]
        match key {
            TagKey::AcoustId => FrameId::ExtendedText("Acoustid Id").into(),
            TagKey::AcoustIdFingerprint => FrameId::ExtendedText("Acoustid Fingerprint").into(),
            TagKey::Album => FrameId::Text("TALB").into(),
            TagKey::AlbumArtist => FrameId::Text("TPE2").into(),
            TagKey::AlbumArtistSortOrder => FrameId::Text("TSO2").into(),
            TagKey::AlbumSortOrder => FrameId::Text("TSOA").into(),
            TagKey::Arranger => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::MultiValuedText("IPLS", "arranger").into(),
                id3::Version::Id3v24 => FrameId::MultiValuedText("TIPL", "arranger").into(),
            },
            TagKey::Artist => FrameId::Text("TPE1").into(),
            TagKey::ArtistSortOrder => FrameId::Text("TSOP").into(),
            TagKey::Artists => FrameId::ExtendedText("ARTISTS").into(),
            TagKey::Asin => FrameId::ExtendedText("ASIN").into(),
            TagKey::Barcode => FrameId::ExtendedText("BARCODE").into(),
            TagKey::Bpm => FrameId::Text("TBPM").into(),
            TagKey::CatalogNumber => FrameId::ExtendedText("CATALOGNUMBER").into(),
            TagKey::Comment => None, // Add mapping to "COMM:description"
            TagKey::Compilation => FrameId::Text("TCMP").into(),
            TagKey::Composer => FrameId::Text("TCOM").into(),
            TagKey::ComposerSortOrder => FrameId::Text("TSOC").into(),
            TagKey::Conductor => FrameId::Text("TPE3").into(),
            TagKey::Copyright => FrameId::Text("TCOP").into(),
            TagKey::Director => FrameId::ExtendedText("DIRECTOR").into(),
            TagKey::DiscNumber => FrameId::Text("TPOS").into(),
            TagKey::DiscSubtitle => match self.data.version() {
                id3::Version::Id3v22 | id3::Version::Id3v23 => None,
                id3::Version::Id3v24 => FrameId::Text("TSST").into(),
            },
            TagKey::EncodedBy => FrameId::Text("TENC").into(),
            TagKey::EncoderSettings => FrameId::Text("TSSE").into(),
            TagKey::Engineer => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::MultiValuedText("IPLS", "engineer").into(),
                id3::Version::Id3v24 => FrameId::MultiValuedText("TIPL", "engineer").into(),
            },
            TagKey::GaplessPlayback => None,
            TagKey::Genre => FrameId::Text("TCON").into(),
            TagKey::Grouping => FrameId::Text("TIT1").into(), // TODO: Add mapping to "GRP1", too?
            TagKey::InitialKey => FrameId::Text("TKEY").into(),
            TagKey::Isrc => FrameId::Text("TSRC").into(),
            TagKey::Language => FrameId::Text("TLAN").into(),
            TagKey::License => None, // TODO: Add mapping to "WCOP" (single URL) or "TXXX:LICENSE" (multiple or non-URL)
            TagKey::Lyricist => FrameId::Text("TEXT").into(),
            TagKey::Lyrics => None, // TODO: Add mapping to USLT:description
            TagKey::Media => FrameId::Text("TMED").into(),
            TagKey::DjMixer => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::MultiValuedText("IPLS", "DJ-mix").into(),
                id3::Version::Id3v24 => FrameId::MultiValuedText("TIPL", "DJ-mix").into(),
            },
            TagKey::Mixer => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::MultiValuedText("IPLS", "mix").into(),
                id3::Version::Id3v24 => FrameId::MultiValuedText("TIPL", "mix").into(),
            },
            TagKey::Mood => match self.data.version() {
                id3::Version::Id3v22 | id3::Version::Id3v23 => None,
                id3::Version::Id3v24 => FrameId::Text("TMOO").into(),
            },
            TagKey::Movement => FrameId::Text("MVNM").into(),
            TagKey::MovementCount => FrameId::Text("MVIN").into(),
            TagKey::MovementNumber => FrameId::Text("MVIN").into(),
            TagKey::MusicBrainzArtistId => FrameId::ExtendedText("MusicBrainz Artist Id").into(),
            TagKey::MusicBrainzDiscId => FrameId::ExtendedText("MusicBrainz Disc Id").into(),
            TagKey::MusicBrainzOriginalArtistId => {
                FrameId::ExtendedText("MusicBrainz Original Artist Id").into()
            }
            TagKey::MusicBrainzOriginalReleaseId => {
                FrameId::ExtendedText("MusicBrainz Original Album Id").into()
            }
            TagKey::MusicBrainzRecordingId => {
                FrameId::UniqueFileIdentifier("http://musicbrainz.org").into()
            }
            TagKey::MusicBrainzReleaseArtistId => {
                FrameId::ExtendedText("MusicBrainz Album Artist Id").into()
            }
            TagKey::MusicBrainzReleaseGroupId => {
                FrameId::ExtendedText("MusicBrainz Release Group Id").into()
            }
            TagKey::MusicBrainzReleaseId => FrameId::ExtendedText("MusicBrainz Album Id").into(),
            TagKey::MusicBrainzTrackId => {
                FrameId::ExtendedText("MusicBrainz Release Track Id").into()
            }
            TagKey::MusicBrainzTrmId => FrameId::ExtendedText("MusicBrainz TRM Id").into(),
            TagKey::MusicBrainzWorkId => FrameId::ExtendedText("MusicBrainz Work Id").into(),
            TagKey::MusicIpFingerprint => FrameId::ExtendedText("MusicMagic Fingerprint").into(),
            TagKey::MusicIpPuid => FrameId::ExtendedText("MusicIP PUID").into(),
            TagKey::OriginalAlbum => FrameId::Text("TOAL").into(),
            TagKey::OriginalArtist => FrameId::Text("TOPE").into(),
            TagKey::OriginalFilename => FrameId::Text("TOFN").into(),
            TagKey::OriginalReleaseDate => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::Text("TORY").into(),
                id3::Version::Id3v24 => FrameId::Text("TDOR").into(),
            },
            TagKey::OriginalReleaseYear => None,
            TagKey::Performer => None, // TODO: Add mapping to "TMCL:instrument" (ID3v2.4) or "IPLS:instrument" (ID3v2.3)
            TagKey::Podcast => None,
            TagKey::PodcastUrl => None,
            TagKey::Producer => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::MultiValuedText("IPLS", "producer").into(),
                id3::Version::Id3v24 => FrameId::MultiValuedText("TIPL", "producer").into(),
            },
            TagKey::Rating => FrameId::Text("POPM").into(),
            TagKey::RecordLabel => FrameId::Text("TPUB").into(),
            TagKey::ReleaseCountry => {
                FrameId::ExtendedText("MusicBrainz Album Release Country").into()
            }
            TagKey::ReleaseDate => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::Text("TDAT").into(),
                id3::Version::Id3v24 => FrameId::Text("TDRC").into(),
            },
            TagKey::ReleaseYear => match self.data.version() {
                id3::Version::Id3v22 | id3::Version::Id3v24 => None,
                id3::Version::Id3v23 => FrameId::Text("TYER").into(),
            },
            TagKey::ReleaseStatus => FrameId::ExtendedText("MusicBrainz Album Status").into(),
            TagKey::ReleaseType => FrameId::ExtendedText("MusicBrainz Album Type").into(),
            TagKey::Remixer => FrameId::Text("TPE4").into(),
            TagKey::ReplayGainAlbumGain => FrameId::ExtendedText("REPLAYGAIN_ALBUM_GAIN").into(),
            TagKey::ReplayGainAlbumPeak => FrameId::ExtendedText("REPLAYGAIN_ALBUM_PEAK").into(),
            TagKey::ReplayGainAlbumRange => FrameId::ExtendedText("REPLAYGAIN_ALBUM_RANGE").into(),
            TagKey::ReplayGainReferenceLoudness => {
                FrameId::ExtendedText("REPLAYGAIN_REFERENCE_LOUDNESS").into()
            }
            TagKey::ReplayGainTrackGain => FrameId::ExtendedText("REPLAYGAIN_TRACK_GAIN").into(),
            TagKey::ReplayGainTrackPeak => FrameId::ExtendedText("REPLAYGAIN_TRACK_PEAK").into(),
            TagKey::ReplayGainTrackRange => FrameId::ExtendedText("REPLAYGAIN_TRACK_RANGE").into(),
            TagKey::Script => FrameId::ExtendedText("SCRIPT").into(),
            TagKey::ShowName => None,
            TagKey::ShowNameSortOrder => None,
            TagKey::ShowMovement => FrameId::ExtendedText("SHOWMOVEMENT").into(),
            TagKey::Subtitle => FrameId::Text("TIT3").into(),
            TagKey::TotalDiscs => FrameId::Text("TPOS").into(),
            TagKey::TotalTracks => FrameId::Text("TRCK").into(),
            TagKey::TrackNumber => FrameId::Text("TRCK").into(),
            TagKey::TrackTitle => FrameId::Text("TIT2").into(),
            TagKey::TrackTitleSortOrder => FrameId::Text("TSOT").into(),
            TagKey::ArtistWebsite => FrameId::Text("WOAR").into(),
            TagKey::WorkTitle => FrameId::ExtendedText("WORK TIT1").into(),
            TagKey::Writer => FrameId::ExtendedText("Writer").into(),
        }
    }

    /// Get the content of a text frame as string.
    fn get_frames<'a>(&'a self, frame_id: &'a str) -> impl Iterator<Item = &'a str> {
        self.data
            .get(frame_id)
            .and_then(|frame| frame.content().text_values())
            .into_iter()
            .flatten()
    }

    /// Get the content of an extended text frame as string.
    fn get_extended_texts<'a>(&'a self, description: &'a str) -> impl Iterator<Item = &'a str> {
        self.data
            .extended_texts()
            .filter(move |extended_text| extended_text.description == description)
            .map(|extended_text| extended_text.value.as_str())
    }

    /// Get the content of multi-valued text frames (e.g., TIPL, IPLS) as string pairs.
    fn get_multi_valued_texts<'a>(
        &'a self,
        frame_id: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a str)> {
        // TODO: Once it becomes stable, `std::iter::Iterator::array_chunks` should be used instead.
        let descriptions = self
            .get_frames(frame_id)
            .enumerate()
            .filter_map(|(i, v)| ((i & 1) == 0).then_some(v));
        let values = self
            .get_frames(frame_id)
            .enumerate()
            .filter_map(|(i, v)| ((i & 1) == 1).then_some(v));
        descriptions.zip(values)
    }

    /// Get the content of unique file identifier frames as byte slices.
    fn get_unique_file_identifiers<'a>(
        &'a self,
        owner_id: &'a str,
    ) -> impl Iterator<Item = &'a [u8]> {
        self.data
            .unique_file_identifiers()
            .filter(move |unique_file_identifier| {
                unique_file_identifier.owner_identifier == owner_id
            })
            .map(|unique_file_identifier| unique_file_identifier.identifier.as_slice())
    }
}

impl Tag for ID3v2Tag {
    fn tag_type(&self) -> TagType {
        match self.data.version() {
            id3::Version::Id3v22 => TagType::ID3v22,
            id3::Version::Id3v23 => TagType::ID3v23,
            id3::Version::Id3v24 => TagType::ID3v24,
        }
    }

    fn get(&self, key: TagKey) -> Option<&str> {
        self.tag_key_to_frame(key)
            .and_then(|frame_id| match frame_id {
                FrameId::Text(id) => self.get_frames(id).next(),
                FrameId::ExtendedText(id) => self.get_extended_texts(id).next(),
                FrameId::UniqueFileIdentifier(id) => self
                    .get_unique_file_identifiers(id)
                    .map(std::str::from_utf8)
                    .find_map(Result::ok),
                FrameId::MultiValuedText(id, desc) => self
                    .get_multi_valued_texts(id)
                    .find_map(|(frame_desc, text)| (frame_desc == desc).then_some(text)),
            })
    }

    fn clear(&mut self, key: TagKey) {
        let frame = self.tag_key_to_frame(key);
        if let Some(frame) = frame {
            match frame {
                #[expect(unused_results)]
                FrameId::Text(id) => {
                    self.data.remove(id);
                }
                FrameId::ExtendedText(description) => {
                    self.data.remove_extended_text(Some(description), None);
                }
                FrameId::UniqueFileIdentifier(owner_id) => {
                    self.data
                        .remove_unique_file_identifier_by_owner_identifier(owner_id);
                }
                FrameId::MultiValuedText(id, desc) => {
                    let new_value = self
                        .get_multi_valued_texts(id)
                        .filter(|(frame_desc, _)| frame_desc != &desc)
                        .fold(String::new(), |acc: String, (desc, text)| {
                            let sep = if acc.is_empty() { "" } else { "\0" };
                            acc + sep + desc + "\0" + text
                        });
                    self.data.set_text(id, new_value);
                }
            }
        }
    }

    fn set(&mut self, key: TagKey, value: Cow<'_, str>) {
        let frame = self.tag_key_to_frame(key);
        if let Some(frame) = frame {
            match frame {
                FrameId::Text(id) => {
                    self.data.set_text(id, value);
                }
                #[expect(unused_results)]
                FrameId::ExtendedText(description) => {
                    self.data.add_frame(ExtendedText {
                        description: description.to_string(),
                        value: value.into_owned(),
                    });
                }
                FrameId::MultiValuedText(id, desc) => {
                    let new_value = self
                        .get_multi_valued_texts(id)
                        .filter(|(frame_desc, _)| frame_desc != &desc)
                        .chain(iter::once((desc, value.borrow())))
                        .fold(String::new(), |acc: String, (desc, text)| {
                            let sep = if acc.is_empty() { "" } else { "\0" };
                            acc + sep + desc + "\0" + text
                        });
                    self.data.set_text(id, new_value);
                }
                #[expect(unused_results)]
                FrameId::UniqueFileIdentifier(owner_id) => {
                    self.data.add_frame(UniqueFileIdentifier {
                        owner_identifier: owner_id.to_string(),
                        identifier: value.as_bytes().to_vec(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag::{Tag, TagKey};
    use id3::Version;
    use paste::paste;

    macro_rules! add_test_get_and_set_with_id3_version {
        ($tagkey:expr, $version:expr, $fnsuffix:ident) => {
            paste! {
                #[test]
                fn [<test_get_and_set_ $fnsuffix>]() {
                    let mut tag = ID3v2Tag::with_version($version);
                    assert!(tag.get($tagkey).is_none());

                    tag.set($tagkey, Cow::from("Example Value"));
                    assert_eq!(tag.get($tagkey), Some("Example Value"));
                }
            }
        };
    }
    macro_rules! add_test_get_and_set_all_id3_versions {
        ($tagkey:pat_param, $fnsuffix:ident) => {
            paste! {
            add_test_get_and_set_with_id3_version!($tagkey, Version::Id3v22, [< $fnsuffix _id3v22>]);
            add_test_get_and_set_with_id3_version!($tagkey, Version::Id3v23, [< $fnsuffix _id3v23>]);
            add_test_get_and_set_with_id3_version!($tagkey, Version::Id3v24, [< $fnsuffix _id3v24>]);
            }
        };
    }

    add_test_get_and_set_all_id3_versions!(TagKey::AcoustId, acoustid);
    add_test_get_and_set_all_id3_versions!(TagKey::AcoustIdFingerprint, acoustidfingerprint);
    add_test_get_and_set_all_id3_versions!(TagKey::Album, album);
    add_test_get_and_set_all_id3_versions!(TagKey::AlbumArtist, albumartist);
    add_test_get_and_set_all_id3_versions!(TagKey::AlbumArtistSortOrder, albumartistsortorder);
    add_test_get_and_set_all_id3_versions!(TagKey::AlbumSortOrder, albumsortorder);
    add_test_get_and_set_all_id3_versions!(TagKey::Artist, artist);
    add_test_get_and_set_all_id3_versions!(TagKey::ArtistSortOrder, artistsortorder);
    add_test_get_and_set_all_id3_versions!(TagKey::Artists, artists);
    add_test_get_and_set_all_id3_versions!(TagKey::Asin, asin);
    add_test_get_and_set_all_id3_versions!(TagKey::Barcode, barcode);
    add_test_get_and_set_all_id3_versions!(TagKey::Bpm, bpm);
    add_test_get_and_set_all_id3_versions!(TagKey::CatalogNumber, catalognumber);
    //add_test_get_and_set_all_id3_versions!(TagKey::Comment, comment);
    add_test_get_and_set_all_id3_versions!(TagKey::Compilation, compilation);
    add_test_get_and_set_all_id3_versions!(TagKey::Composer, composer);
    add_test_get_and_set_all_id3_versions!(TagKey::ComposerSortOrder, composersortorder);
    add_test_get_and_set_all_id3_versions!(TagKey::Conductor, conductor);
    add_test_get_and_set_all_id3_versions!(TagKey::Copyright, copyright);
    add_test_get_and_set_all_id3_versions!(TagKey::Director, director);
    add_test_get_and_set_all_id3_versions!(TagKey::DiscNumber, discnumber);
    add_test_get_and_set_with_id3_version!(
        TagKey::DiscSubtitle,
        Version::Id3v24,
        discsubtitle_id3v24
    );
    add_test_get_and_set_all_id3_versions!(TagKey::EncodedBy, encodedby);
    add_test_get_and_set_all_id3_versions!(TagKey::EncoderSettings, encodersettings);
    //add_test_get_and_set_all_id3_versions!(TagKey::GaplessPlayback, gaplessplayback);
    add_test_get_and_set_all_id3_versions!(TagKey::Genre, genre);
    add_test_get_and_set_all_id3_versions!(TagKey::Grouping, grouping);
    add_test_get_and_set_all_id3_versions!(TagKey::InitialKey, initialkey);
    add_test_get_and_set_all_id3_versions!(TagKey::Isrc, isrc);
    add_test_get_and_set_all_id3_versions!(TagKey::Language, language);
    //add_test_get_and_set_all_id3_versions!(TagKey::License, license);
    add_test_get_and_set_all_id3_versions!(TagKey::Lyricist, lyricist);
    //add_test_get_and_set_all_id3_versions!(TagKey::Lyrics, lyrics);
    add_test_get_and_set_all_id3_versions!(TagKey::Media, media);
    add_test_get_and_set_with_id3_version!(TagKey::Mood, Version::Id3v24, mood_id3v24);
    add_test_get_and_set_all_id3_versions!(TagKey::Movement, movement);
    add_test_get_and_set_all_id3_versions!(TagKey::MovementCount, movementcount);
    add_test_get_and_set_all_id3_versions!(TagKey::MovementNumber, movementnumber);
    add_test_get_and_set_all_id3_versions!(TagKey::MusicBrainzArtistId, musicbrainzartistid);
    add_test_get_and_set_all_id3_versions!(TagKey::MusicBrainzDiscId, musicbrainzdiscid);
    add_test_get_and_set_all_id3_versions!(
        TagKey::MusicBrainzOriginalArtistId,
        musicbrainzoriginalartistid
    );
    add_test_get_and_set_all_id3_versions!(
        TagKey::MusicBrainzOriginalReleaseId,
        musicbrainzoriginalreleaseid
    );
    add_test_get_and_set_all_id3_versions!(TagKey::MusicBrainzRecordingId, musicbrainzrecordingid);
    add_test_get_and_set_all_id3_versions!(
        TagKey::MusicBrainzReleaseArtistId,
        musicbrainzreleaseartistid
    );
    add_test_get_and_set_all_id3_versions!(
        TagKey::MusicBrainzReleaseGroupId,
        musicbrainzreleasegroupid
    );
    add_test_get_and_set_all_id3_versions!(TagKey::MusicBrainzReleaseId, musicbrainzreleaseid);
    add_test_get_and_set_all_id3_versions!(TagKey::MusicBrainzTrackId, musicbrainztrackid);
    add_test_get_and_set_all_id3_versions!(TagKey::MusicBrainzTrmId, musicbrainztrmid);
    add_test_get_and_set_all_id3_versions!(TagKey::MusicBrainzWorkId, musicbrainzworkid);
    add_test_get_and_set_all_id3_versions!(TagKey::MusicIpFingerprint, musicipfingerprint);
    add_test_get_and_set_all_id3_versions!(TagKey::MusicIpPuid, musicippuid);
    add_test_get_and_set_all_id3_versions!(TagKey::OriginalAlbum, originalalbum);
    add_test_get_and_set_all_id3_versions!(TagKey::OriginalArtist, originalartist);
    add_test_get_and_set_all_id3_versions!(TagKey::OriginalFilename, originalfilename);
    add_test_get_and_set_with_id3_version!(
        TagKey::OriginalReleaseDate,
        Version::Id3v23,
        originalreleasedate_id3v23
    );
    add_test_get_and_set_with_id3_version!(
        TagKey::OriginalReleaseDate,
        Version::Id3v24,
        originalreleasedate_id3v24
    );
    //add_test_get_and_set_all_id3_versions!(TagKey::OriginalReleaseYear, originalreleaseyear);
    //add_test_get_and_set_all_id3_versions!(TagKey::Performer, performer);
    //add_test_get_and_set_all_id3_versions!(TagKey::Podcast, podcast);
    //add_test_get_and_set_all_id3_versions!(TagKey::PodcastUrl, podcasturl);
    add_test_get_and_set_all_id3_versions!(TagKey::Rating, rating);
    add_test_get_and_set_all_id3_versions!(TagKey::RecordLabel, recordlabel);
    add_test_get_and_set_all_id3_versions!(TagKey::ReleaseCountry, releasecountry);
    add_test_get_and_set_with_id3_version!(
        TagKey::ReleaseDate,
        Version::Id3v23,
        releasedate_id3v23
    );
    add_test_get_and_set_with_id3_version!(
        TagKey::ReleaseDate,
        Version::Id3v24,
        releasedate_id3v24
    );
    add_test_get_and_set_with_id3_version!(
        TagKey::ReleaseYear,
        Version::Id3v23,
        releaseyear_id3v23
    );
    add_test_get_and_set_all_id3_versions!(TagKey::ReleaseStatus, releasestatus);
    add_test_get_and_set_all_id3_versions!(TagKey::ReleaseType, releasetype);
    add_test_get_and_set_all_id3_versions!(TagKey::Remixer, remixer);
    add_test_get_and_set_all_id3_versions!(TagKey::ReplayGainAlbumGain, replaygainalbumgain);
    add_test_get_and_set_all_id3_versions!(TagKey::ReplayGainAlbumPeak, replaygainalbumpeak);
    add_test_get_and_set_all_id3_versions!(TagKey::ReplayGainAlbumRange, replaygainalbumrange);
    add_test_get_and_set_all_id3_versions!(
        TagKey::ReplayGainReferenceLoudness,
        replaygainreferenceloudness
    );
    add_test_get_and_set_all_id3_versions!(TagKey::ReplayGainTrackGain, replaygaintrackgain);
    add_test_get_and_set_all_id3_versions!(TagKey::ReplayGainTrackPeak, replaygaintrackpeak);
    add_test_get_and_set_all_id3_versions!(TagKey::ReplayGainTrackRange, replaygaintrackrange);
    add_test_get_and_set_all_id3_versions!(TagKey::Script, script);
    //add_test_get_and_set_all_id3_versions!(TagKey::ShowName, showname);
    //add_test_get_and_set_all_id3_versions!(TagKey::ShowNameSortOrder, shownamesortorder);
    add_test_get_and_set_all_id3_versions!(TagKey::ShowMovement, showmovement);
    add_test_get_and_set_all_id3_versions!(TagKey::Subtitle, subtitle);
    add_test_get_and_set_all_id3_versions!(TagKey::TotalDiscs, totaldiscs);
    add_test_get_and_set_all_id3_versions!(TagKey::TotalTracks, totaltracks);
    add_test_get_and_set_all_id3_versions!(TagKey::TrackNumber, tracknumber);
    add_test_get_and_set_all_id3_versions!(TagKey::TrackTitle, tracktitle);
    add_test_get_and_set_all_id3_versions!(TagKey::TrackTitleSortOrder, tracktitlesortorder);
    add_test_get_and_set_all_id3_versions!(TagKey::ArtistWebsite, artistwebsite);
    add_test_get_and_set_all_id3_versions!(TagKey::WorkTitle, worktitle);
    add_test_get_and_set_all_id3_versions!(TagKey::Writer, writer);

    #[test]
    fn test_clear() {
        let mut tag = ID3v2Tag::new();
        assert!(tag.get(TagKey::Genre).is_none());

        tag.set(TagKey::Genre, Cow::from("Hard Bop"));
        assert!(tag.get(TagKey::Genre).is_some());

        tag.clear(TagKey::Genre);
        assert!(tag.get(TagKey::Genre).is_none());
    }

    #[test]
    fn test_set_or_clear_some() {
        let mut tag = ID3v2Tag::new();
        assert!(tag.get(TagKey::Genre).is_none());

        tag.set_or_clear(TagKey::Genre, Some(Cow::from("Hard Bop")));
        assert!(tag.get(TagKey::Genre).is_some());

        tag.set_or_clear(TagKey::Genre, Some(Cow::from("Jazz")));
        assert!(tag.get(TagKey::Genre).is_some());

        tag.set_or_clear(TagKey::Genre, None);
        assert!(tag.get(TagKey::Genre).is_none());
    }

    #[test]
    fn test_get_and_set_multivalued_text() {
        let mut tag = ID3v2Tag::new();
        assert!(tag.get(TagKey::Arranger).is_none());
        assert!(tag.get(TagKey::Engineer).is_none());
        assert!(tag.get(TagKey::DjMixer).is_none());
        assert!(tag.get(TagKey::Mixer).is_none());
        assert!(tag.get(TagKey::Producer).is_none());

        tag.set(TagKey::Arranger, Cow::from("An awesome Arranger"));

        assert_eq!(tag.get(TagKey::Arranger), Some("An awesome Arranger"));
        assert!(tag.get(TagKey::Engineer).is_none());
        assert!(tag.get(TagKey::DjMixer).is_none());
        assert!(tag.get(TagKey::Mixer).is_none());
        assert!(tag.get(TagKey::Producer).is_none());

        tag.set(TagKey::Engineer, Cow::from("Mrs. Engineer"));

        assert_eq!(tag.get(TagKey::Arranger), Some("An awesome Arranger"));
        assert_eq!(tag.get(TagKey::Engineer), Some("Mrs. Engineer"));
        assert!(tag.get(TagKey::DjMixer).is_none());
        assert!(tag.get(TagKey::Mixer).is_none());
        assert!(tag.get(TagKey::Producer).is_none());

        tag.set(TagKey::DjMixer, Cow::from("Mr. DJ"));

        assert_eq!(tag.get(TagKey::Arranger), Some("An awesome Arranger"));
        assert_eq!(tag.get(TagKey::Engineer), Some("Mrs. Engineer"));
        assert_eq!(tag.get(TagKey::DjMixer), Some("Mr. DJ"));
        assert!(tag.get(TagKey::Mixer).is_none());
        assert!(tag.get(TagKey::Producer).is_none());

        tag.set(TagKey::Mixer, Cow::from("Miss Mixer"));

        assert_eq!(tag.get(TagKey::Arranger), Some("An awesome Arranger"));
        assert_eq!(tag.get(TagKey::Engineer), Some("Mrs. Engineer"));
        assert_eq!(tag.get(TagKey::DjMixer), Some("Mr. DJ"));
        assert_eq!(tag.get(TagKey::Mixer), Some("Miss Mixer"));
        assert!(tag.get(TagKey::Producer).is_none());

        tag.set(TagKey::Producer, Cow::from("Producer Dude"));

        assert_eq!(tag.get(TagKey::Arranger), Some("An awesome Arranger"));
        assert_eq!(tag.get(TagKey::Engineer), Some("Mrs. Engineer"));
        assert_eq!(tag.get(TagKey::DjMixer), Some("Mr. DJ"));
        assert_eq!(tag.get(TagKey::Mixer), Some("Miss Mixer"));
        assert_eq!(tag.get(TagKey::Producer), Some("Producer Dude"));

        tag.clear(TagKey::DjMixer);

        assert_eq!(tag.get(TagKey::Arranger), Some("An awesome Arranger"));
        assert_eq!(tag.get(TagKey::Engineer), Some("Mrs. Engineer"));
        assert!(tag.get(TagKey::DjMixer).is_none());
        assert_eq!(tag.get(TagKey::Mixer), Some("Miss Mixer"));
        assert_eq!(tag.get(TagKey::Producer), Some("Producer Dude"));

        tag.clear(TagKey::Arranger);

        assert!(tag.get(TagKey::Arranger).is_none());
        assert_eq!(tag.get(TagKey::Engineer), Some("Mrs. Engineer"));
        assert!(tag.get(TagKey::DjMixer).is_none());
        assert_eq!(tag.get(TagKey::Mixer), Some("Miss Mixer"));
        assert_eq!(tag.get(TagKey::Producer), Some("Producer Dude"));

        tag.set_or_clear(TagKey::Mixer, None);

        assert!(tag.get(TagKey::Arranger).is_none());
        assert_eq!(tag.get(TagKey::Engineer), Some("Mrs. Engineer"));
        assert!(tag.get(TagKey::DjMixer).is_none());
        assert!(tag.get(TagKey::Mixer).is_none());
        assert_eq!(tag.get(TagKey::Producer), Some("Producer Dude"));
    }
}
