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
    frame::{Comment, ExtendedText, UniqueFileIdentifier},
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
    /// Comment frame (`COMM`).
    Comment(&'a str),
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
            TagKey::Comment => FrameId::Comment("description").into(), // TODO: Check if "description" is meant literally here.
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
                FrameId::Comment(desc) => self.data.comments().find_map(|comment| {
                    if comment.lang == "eng" && comment.description == desc {
                        Some(comment.text.as_str())
                    } else {
                        None
                    }
                }),
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
                #[expect(unused_results)]
                FrameId::MultiValuedText(id, desc) => {
                    let new_value = self
                        .get_multi_valued_texts(id)
                        .filter(|(frame_desc, _)| frame_desc != &desc)
                        .fold(String::new(), |acc: String, (desc, text)| {
                            let sep = if acc.is_empty() { "" } else { "\0" };
                            acc + sep + desc + "\0" + text
                        });
                    if new_value.is_empty() {
                        self.data.remove(id);
                    } else {
                        self.data.set_text(id, new_value);
                    }
                }
                FrameId::Comment(desc) => {
                    self.data.remove_comment(Some(desc), None);
                }
            }
        }
    }

    fn set_multiple<'a>(&'a mut self, key: TagKey, values: &[Cow<'a, str>]) {
        if values.is_empty() {
            self.clear(key);
            return;
        }

        let frame = self.tag_key_to_frame(key);
        match frame {
            Some(FrameId::MultiValuedText(id, desc)) => {
                let new_value = self
                    .get_multi_valued_texts(id)
                    .filter(|(frame_desc, _)| frame_desc != &desc)
                    .chain(values.iter().map(|value| (desc, value.borrow())))
                    .fold(String::new(), |acc: String, (desc, text)| {
                        let sep = if acc.is_empty() { "" } else { "\0" };
                        acc + sep + desc + "\0" + text
                    });
                self.data.set_text(id, new_value);
            }
            _ => {
                self.set(key, values.join(" / ").into());
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
                #[expect(unused_results)]
                FrameId::Comment(desc) => {
                    self.data.add_frame(Comment {
                        lang: "eng".to_string(),
                        description: desc.to_string(),
                        text: value.to_string(),
                    });
                }
            }
        }
    }

    fn write(&mut self, path: &Path) -> crate::Result<()> {
        self.data.write_to_path(path, self.data.version())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag::{Tag, TagKey};
    use id3::Version;
    use paste::paste;

    #[test]
    fn test_tag_type_id3v22() {
        let tag = ID3v2Tag::with_version(Version::Id3v22);
        assert_eq!(tag.tag_type(), TagType::ID3v22);
    }

    #[test]
    fn test_tag_type_id3v23() {
        let tag = ID3v2Tag::with_version(Version::Id3v23);
        assert_eq!(tag.tag_type(), TagType::ID3v23);
    }

    #[test]
    fn test_tag_type_id3v24() {
        let tag = ID3v2Tag::with_version(Version::Id3v24);
        assert_eq!(tag.tag_type(), TagType::ID3v24);
    }

    #[test]
    fn test_get_set_clear_multivalued_text() {
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

    macro_rules! add_tests_with_id3_version {
        ($tagkey:expr, $version:expr, $fnsuffix:ident) => {
            paste! {
                #[test]
                fn [<test_get_set_ $fnsuffix>]() {
                    let mut tag = ID3v2Tag::with_version($version);
                    assert!(tag.get($tagkey).is_none());

                    tag.set($tagkey, Cow::from("Example Value"));
                    assert_eq!(tag.get($tagkey), Some("Example Value"));
                }

                #[test]
                fn [<test_clear_ $fnsuffix>]() {
                    let mut tag = ID3v2Tag::with_version($version);
                    assert!(tag.get($tagkey).is_none());

                    tag.set($tagkey, Cow::from("Example Value"));
                    assert!(tag.get($tagkey).is_some());

                    tag.clear($tagkey);
                    assert!(tag.get($tagkey).is_none());
                }

                #[test]
                fn [<test_set_or_clear_ $fnsuffix>]() {
                    let mut tag = ID3v2Tag::with_version($version);
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
    macro_rules! add_tests_with_id3_versions_all {
        ($tagkey:pat_param, $fnsuffix:ident) => {
            paste! {
                add_tests_with_id3_version!($tagkey, Version::Id3v22, [< $fnsuffix _id3v22>]);
                add_tests_with_id3_version!($tagkey, Version::Id3v23, [< $fnsuffix _id3v23>]);
                add_tests_with_id3_version!($tagkey, Version::Id3v24, [< $fnsuffix _id3v24>]);
            }
        };
    }

    add_tests_with_id3_versions_all!(TagKey::AcoustId, acoustid);
    add_tests_with_id3_versions_all!(TagKey::AcoustIdFingerprint, acoustidfingerprint);
    add_tests_with_id3_versions_all!(TagKey::Album, album);
    add_tests_with_id3_versions_all!(TagKey::AlbumArtist, albumartist);
    add_tests_with_id3_versions_all!(TagKey::AlbumArtistSortOrder, albumartistsortorder);
    add_tests_with_id3_versions_all!(TagKey::AlbumSortOrder, albumsortorder);
    add_tests_with_id3_versions_all!(TagKey::Artist, artist);
    add_tests_with_id3_versions_all!(TagKey::ArtistSortOrder, artistsortorder);
    add_tests_with_id3_versions_all!(TagKey::Artists, artists);
    add_tests_with_id3_versions_all!(TagKey::Asin, asin);
    add_tests_with_id3_versions_all!(TagKey::Barcode, barcode);
    add_tests_with_id3_versions_all!(TagKey::Bpm, bpm);
    add_tests_with_id3_versions_all!(TagKey::CatalogNumber, catalognumber);
    add_tests_with_id3_versions_all!(TagKey::Comment, comment);
    add_tests_with_id3_versions_all!(TagKey::Compilation, compilation);
    add_tests_with_id3_versions_all!(TagKey::Composer, composer);
    add_tests_with_id3_versions_all!(TagKey::ComposerSortOrder, composersortorder);
    add_tests_with_id3_versions_all!(TagKey::Conductor, conductor);
    add_tests_with_id3_versions_all!(TagKey::Copyright, copyright);
    add_tests_with_id3_versions_all!(TagKey::Director, director);
    add_tests_with_id3_versions_all!(TagKey::DiscNumber, discnumber);
    add_tests_with_id3_version!(TagKey::DiscSubtitle, Version::Id3v24, discsubtitle_id3v24);
    add_tests_with_id3_versions_all!(TagKey::EncodedBy, encodedby);
    add_tests_with_id3_versions_all!(TagKey::EncoderSettings, encodersettings);
    //add_tests_with_id3_versions_all!(TagKey::GaplessPlayback, gaplessplayback);
    add_tests_with_id3_versions_all!(TagKey::Genre, genre);
    add_tests_with_id3_versions_all!(TagKey::Grouping, grouping);
    add_tests_with_id3_versions_all!(TagKey::InitialKey, initialkey);
    add_tests_with_id3_versions_all!(TagKey::Isrc, isrc);
    add_tests_with_id3_versions_all!(TagKey::Language, language);
    //add_tests_with_id3_versions_all!(TagKey::License, license);
    add_tests_with_id3_versions_all!(TagKey::Lyricist, lyricist);
    //add_tests_with_id3_versions_all!(TagKey::Lyrics, lyrics);
    add_tests_with_id3_versions_all!(TagKey::Media, media);
    add_tests_with_id3_version!(TagKey::Mood, Version::Id3v24, mood_id3v24);
    add_tests_with_id3_versions_all!(TagKey::Movement, movement);
    add_tests_with_id3_versions_all!(TagKey::MovementCount, movementcount);
    add_tests_with_id3_versions_all!(TagKey::MovementNumber, movementnumber);
    add_tests_with_id3_versions_all!(TagKey::MusicBrainzArtistId, musicbrainzartistid);
    add_tests_with_id3_versions_all!(TagKey::MusicBrainzDiscId, musicbrainzdiscid);
    add_tests_with_id3_versions_all!(
        TagKey::MusicBrainzOriginalArtistId,
        musicbrainzoriginalartistid
    );
    add_tests_with_id3_versions_all!(
        TagKey::MusicBrainzOriginalReleaseId,
        musicbrainzoriginalreleaseid
    );
    add_tests_with_id3_versions_all!(TagKey::MusicBrainzRecordingId, musicbrainzrecordingid);
    add_tests_with_id3_versions_all!(
        TagKey::MusicBrainzReleaseArtistId,
        musicbrainzreleaseartistid
    );
    add_tests_with_id3_versions_all!(TagKey::MusicBrainzReleaseGroupId, musicbrainzreleasegroupid);
    add_tests_with_id3_versions_all!(TagKey::MusicBrainzReleaseId, musicbrainzreleaseid);
    add_tests_with_id3_versions_all!(TagKey::MusicBrainzTrackId, musicbrainztrackid);
    add_tests_with_id3_versions_all!(TagKey::MusicBrainzTrmId, musicbrainztrmid);
    add_tests_with_id3_versions_all!(TagKey::MusicBrainzWorkId, musicbrainzworkid);
    add_tests_with_id3_versions_all!(TagKey::MusicIpFingerprint, musicipfingerprint);
    add_tests_with_id3_versions_all!(TagKey::MusicIpPuid, musicippuid);
    add_tests_with_id3_versions_all!(TagKey::OriginalAlbum, originalalbum);
    add_tests_with_id3_versions_all!(TagKey::OriginalArtist, originalartist);
    add_tests_with_id3_versions_all!(TagKey::OriginalFilename, originalfilename);
    add_tests_with_id3_version!(
        TagKey::OriginalReleaseDate,
        Version::Id3v23,
        originalreleasedate_id3v23
    );
    add_tests_with_id3_version!(
        TagKey::OriginalReleaseDate,
        Version::Id3v24,
        originalreleasedate_id3v24
    );
    //add_tests_with_id3_versions_all!(TagKey::OriginalReleaseYear, originalreleaseyear);
    //add_tests_with_id3_versions_all!(TagKey::Performer, performer);
    //add_tests_with_id3_versions_all!(TagKey::Podcast, podcast);
    //add_tests_with_id3_versions_all!(TagKey::PodcastUrl, podcasturl);
    add_tests_with_id3_versions_all!(TagKey::Rating, rating);
    add_tests_with_id3_versions_all!(TagKey::RecordLabel, recordlabel);
    add_tests_with_id3_versions_all!(TagKey::ReleaseCountry, releasecountry);
    add_tests_with_id3_version!(TagKey::ReleaseDate, Version::Id3v23, releasedate_id3v23);
    add_tests_with_id3_version!(TagKey::ReleaseDate, Version::Id3v24, releasedate_id3v24);
    add_tests_with_id3_version!(TagKey::ReleaseYear, Version::Id3v23, releaseyear_id3v23);
    add_tests_with_id3_versions_all!(TagKey::ReleaseStatus, releasestatus);
    add_tests_with_id3_versions_all!(TagKey::ReleaseType, releasetype);
    add_tests_with_id3_versions_all!(TagKey::Remixer, remixer);
    add_tests_with_id3_versions_all!(TagKey::ReplayGainAlbumGain, replaygainalbumgain);
    add_tests_with_id3_versions_all!(TagKey::ReplayGainAlbumPeak, replaygainalbumpeak);
    add_tests_with_id3_versions_all!(TagKey::ReplayGainAlbumRange, replaygainalbumrange);
    add_tests_with_id3_versions_all!(
        TagKey::ReplayGainReferenceLoudness,
        replaygainreferenceloudness
    );
    add_tests_with_id3_versions_all!(TagKey::ReplayGainTrackGain, replaygaintrackgain);
    add_tests_with_id3_versions_all!(TagKey::ReplayGainTrackPeak, replaygaintrackpeak);
    add_tests_with_id3_versions_all!(TagKey::ReplayGainTrackRange, replaygaintrackrange);
    add_tests_with_id3_versions_all!(TagKey::Script, script);
    //add_tests_with_id3_versions_all!(TagKey::ShowName, showname);
    //add_tests_with_id3_versions_all!(TagKey::ShowNameSortOrder, shownamesortorder);
    add_tests_with_id3_versions_all!(TagKey::ShowMovement, showmovement);
    add_tests_with_id3_versions_all!(TagKey::Subtitle, subtitle);
    add_tests_with_id3_versions_all!(TagKey::TotalDiscs, totaldiscs);
    add_tests_with_id3_versions_all!(TagKey::TotalTracks, totaltracks);
    add_tests_with_id3_versions_all!(TagKey::TrackNumber, tracknumber);
    add_tests_with_id3_versions_all!(TagKey::TrackTitle, tracktitle);
    add_tests_with_id3_versions_all!(TagKey::TrackTitleSortOrder, tracktitlesortorder);
    add_tests_with_id3_versions_all!(TagKey::ArtistWebsite, artistwebsite);
    add_tests_with_id3_versions_all!(TagKey::WorkTitle, worktitle);
    add_tests_with_id3_versions_all!(TagKey::Writer, writer);
}
