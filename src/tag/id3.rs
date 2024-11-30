// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Support for ID3 tags.

use crate::tag::{Tag, TagKey, TagType};
use crate::track::InvolvedPerson;
use crate::util::parse_year_from_str;
use id3::{
    frame::{
        Comment, ExtendedText, Frame, InvolvedPeopleList, InvolvedPeopleListItem,
        UniqueFileIdentifier,
    },
    Content, TagLike,
};
use std::borrow::Cow;
use std::iter;
use std::mem;
use std::path::Path;

/// Determines which part of an combined text part.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum CombinedTextPart {
    /// The first value.
    First,
    /// The second value.
    Second,
}

/// ID3 frame ID.
#[derive(Debug)]
enum FrameId<'a> {
    /// Text frame.
    Text(&'a str),
    /// Special kind of text frame, where multiple values are joined with a slash. The second field is the index.
    CombinedText(&'a str, CombinedTextPart),
    /// Extended Text frame (`TXXX`).
    ExtendedText(&'a str),
    /// Unique File Identifier frame (`UFID`).
    UniqueFileIdentifier(&'a str),
    /// Comment frame (`COMM`).
    Comment(&'a str),
    /// Involved Person List in a `IPLS`/`TMCL`/`TIPL` frame.
    InvolvedPersonList(&'a str),
    /// Involved Person in a `IPLS`/`TMCL`/`TIPL` frame.
    InvolvedPerson(&'a str, &'a str),
    /// A valued derived from another tag.
    DerivedValue(TagKey, fn(&str) -> Option<String>),
}

const IPLS_NON_PERFORMER_INVOLVEMENTS: [&str; 5] =
    ["arranger", "engineer", "DJ-mix", "mix", "producer"];

/// ID3 tag (version 2).
#[derive(Debug)]
pub struct ID3v2Tag {
    /// The underlying tag data.
    data: id3::Tag,
}

impl ID3v2Tag {
    #[cfg(test)]
    pub fn with_version(version: id3::Version) -> Self {
        ID3v2Tag {
            data: id3::Tag::with_version(version),
        }
    }

    /// Read the ID3 tag from the path
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        let data = id3::Tag::read_from_path(path).or_else(|err| {
            if matches!(err.kind, id3::ErrorKind::NoTag) {
                Ok(id3::Tag::new())
            } else {
                Err(err)
            }
        })?;
        Ok(ID3v2Tag { data })
    }

    /// Get the ID3 frame for a tag key.
    fn tag_key_to_frame<'a>(&self, key: &'a TagKey) -> Option<FrameId<'a>> {
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
                id3::Version::Id3v23 => FrameId::InvolvedPerson("IPLS", "arranger").into(),
                id3::Version::Id3v24 => FrameId::InvolvedPerson("TIPL", "arranger").into(),
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
            TagKey::DiscNumber => FrameId::CombinedText("TPOS", CombinedTextPart::First).into(),
            TagKey::DiscSubtitle => match self.data.version() {
                id3::Version::Id3v22 | id3::Version::Id3v23 => None,
                id3::Version::Id3v24 => FrameId::Text("TSST").into(),
            },
            TagKey::EncodedBy => FrameId::Text("TENC").into(),
            TagKey::EncoderSettings => FrameId::Text("TSSE").into(),
            TagKey::Engineer => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::InvolvedPerson("IPLS", "engineer").into(),
                id3::Version::Id3v24 => FrameId::InvolvedPerson("TIPL", "engineer").into(),
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
                id3::Version::Id3v23 => FrameId::InvolvedPerson("IPLS", "DJ-mix").into(),
                id3::Version::Id3v24 => FrameId::InvolvedPerson("TIPL", "DJ-mix").into(),
            },
            TagKey::Mixer => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::InvolvedPerson("IPLS", "mix").into(),
                id3::Version::Id3v24 => FrameId::InvolvedPerson("TIPL", "mix").into(),
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
            TagKey::OriginalReleaseYear => {
                FrameId::DerivedValue(TagKey::OriginalReleaseDate, parse_year_from_str).into()
            }
            TagKey::Performers => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::InvolvedPersonList("IPLS").into(),
                id3::Version::Id3v24 => FrameId::InvolvedPersonList("TMCL").into(),
            },
            TagKey::Performer(instrument) => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::InvolvedPerson("IPLS", instrument).into(),
                id3::Version::Id3v24 => FrameId::InvolvedPerson("TMCL", instrument).into(),
            },
            TagKey::Podcast => None,
            TagKey::PodcastUrl => None,
            TagKey::Producer => match self.data.version() {
                id3::Version::Id3v22 => None,
                id3::Version::Id3v23 => FrameId::InvolvedPerson("IPLS", "producer").into(),
                id3::Version::Id3v24 => FrameId::InvolvedPerson("TIPL", "producer").into(),
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
                id3::Version::Id3v22 => None,
                id3::Version::Id3v24 => {
                    FrameId::DerivedValue(TagKey::ReleaseDate, parse_year_from_str).into()
                }
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
            TagKey::TotalDiscs => FrameId::CombinedText("TPOS", CombinedTextPart::Second).into(),
            TagKey::TotalTracks => FrameId::CombinedText("TRCK", CombinedTextPart::Second).into(),
            TagKey::TrackNumber => FrameId::CombinedText("TRCK", CombinedTextPart::First).into(),
            TagKey::TrackTitle => FrameId::Text("TIT2").into(),
            TagKey::TrackTitleSortOrder => FrameId::Text("TSOT").into(),
            TagKey::ArtistWebsite => FrameId::Text("WOAR").into(),
            TagKey::WorkTitle => FrameId::ExtendedText("WORK").into(), // TODO: Add mapping to "TIT1", too?

            TagKey::Writer => FrameId::ExtendedText("Writer").into(),
        }
    }

    /// Get the content of a text frame as string.
    fn get_frames<'a>(&'a self, frame_id: &'a str) -> impl Iterator<Item = &'a str> {
        self.data
            .get(frame_id)
            .and_then(|frame| {
                let mut text_values = None;
                match frame.content() {
                    Content::Text(_) | Content::ExtendedText(_) => {
                        text_values = frame.content().text_values();
                    }
                    Content::Unknown(unknown) => {
                        println!("{:02x?}", &unknown.data);
                    }
                    _ => (),
                };
                //let use_unknown = text_values.is_none();
                //text_values.into_iter().map(|it| it.chain(frame.content().to_unknown().ok().filter(|_| use_unknown).into_iter().filter_map(|unk| String::from_utf8(unk.data).ok())))
                text_values
            })
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

    /// Get the combined text part as string.
    fn get_combined_text_part<'a>(
        &'a self,
        id: &'a str,
        part: CombinedTextPart,
    ) -> Option<&'a str> {
        self.get_frames(id).next().and_then(|value| {
            value
                .split_once('/')
                .map(|(first, second)| match part {
                    CombinedTextPart::First => first,
                    CombinedTextPart::Second => second,
                })
                .or_else(|| (part == CombinedTextPart::First).then_some(value))
        })
    }

    /// Migrate this tag to the given ID3 version.
    pub fn migrate_to(&mut self, new_version: id3::Version) {
        let version = self.data.version();
        if version == new_version {
            return;
        }

        // FIXME: Converting to ID3v2.2 is not supported.
        if new_version == id3::Version::Id3v22 {
            return;
        }

        log::info!("Converting ID3 tag version {version} to {new_version}");

        let old_data = mem::replace(&mut self.data, id3::Tag::with_version(new_version));
        for frame in old_data.frames() {
            let id = frame.id();
            let new_frame = match frame.id_for_version(new_version) {
                Some(new_id) if new_id == id => frame.clone(),
                Some(new_id) => {
                    log::info!("Converting ID3 frame {id} to {new_id}");
                    let content = frame.content().to_owned();
                    Frame::with_content(new_id, content)
                }
                None => {
                    log::info!("Removing unsupported ID3 frame {id}");
                    continue;
                }
            };
            let _unused = self.data.add_frame(new_frame);
        }
    }
}

impl Default for ID3v2Tag {
    fn default() -> Self {
        ID3v2Tag {
            data: id3::Tag::with_version(id3::Version::Id3v23),
        }
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

    fn get<'a>(&'a self, key: &'a TagKey) -> Option<Cow<'a, str>> {
        self.tag_key_to_frame(key)
            .and_then(|frame_id| match frame_id {
                FrameId::Text(id) => self.get_frames(id).map(Cow::from).next(),
                FrameId::CombinedText(id, part) => {
                    self.get_combined_text_part(id, part).map(Cow::from)
                }
                FrameId::ExtendedText(id) => self
                    .get_extended_texts(id)
                    .map(|value| {
                        value
                            .char_indices()
                            .next_back()
                            .map_or(value, |(i, character)| {
                                if character == '\0' {
                                    &value[..i]
                                } else {
                                    value
                                }
                            })
                    })
                    .map(Cow::from)
                    .next(),
                FrameId::UniqueFileIdentifier(id) => self
                    .get_unique_file_identifiers(id)
                    .map(std::str::from_utf8)
                    .find_map(Result::ok)
                    .map(Cow::from),
                FrameId::Comment(desc) => self.data.comments().find_map(|comment| {
                    if comment.lang == "eng" && comment.description == desc {
                        Some(Cow::from(comment.text.as_str()))
                    } else {
                        None
                    }
                }),
                FrameId::InvolvedPersonList(_) => {
                    // Use the dedicated `performers()` method instead.
                    unreachable!();
                }
                FrameId::InvolvedPerson(id, involvement) => self
                    .data
                    .get(id)
                    .and_then(|frame| frame.content().involved_people_list())
                    .and_then(|people_list| {
                        people_list.items.iter().find_map(|item| {
                            (item.involvement.as_str() == involvement)
                                .then_some(item.involvee.as_str())
                        })
                    })
                    .map(Cow::from),
                FrameId::DerivedValue(original_key, derive_func) => self
                    .get(&original_key)
                    .as_deref()
                    .and_then(derive_func)
                    .map(Cow::from),
            })
    }

    fn clear(&mut self, key: &TagKey) {
        let frame = self.tag_key_to_frame(key);
        if let Some(frame) = frame {
            match frame {
                FrameId::Text(id)
                | FrameId::CombinedText(id, CombinedTextPart::First)
                | FrameId::InvolvedPersonList(id) => {
                    let _unused = self.data.remove(id);
                }
                FrameId::CombinedText(id, CombinedTextPart::Second) => {
                    if let Some(value) = self
                        .get_combined_text_part(id, CombinedTextPart::First)
                        .map(ToOwned::to_owned)
                    {
                        self.data.set_text(id, value);
                    } else {
                        let _unused = self.data.remove(id);
                    }
                }
                FrameId::ExtendedText(description) => {
                    self.data.remove_extended_text(Some(description), None);
                }
                FrameId::UniqueFileIdentifier(owner_id) => {
                    self.data
                        .remove_unique_file_identifier_by_owner_identifier(owner_id);
                }
                FrameId::Comment(desc) => {
                    self.data.remove_comment(Some(desc), None);
                }
                FrameId::InvolvedPerson(id, involvement) => {
                    let remaining_items = self
                        .data
                        .get(id)
                        .and_then(|frame| frame.content().involved_people_list())
                        .map(|people_list| {
                            people_list
                                .items
                                .clone()
                                .into_iter()
                                .filter(|item| item.involvement.as_str() != involvement)
                                .collect::<Vec<_>>()
                        });
                    let _unused = self.data.remove(id);
                    if let Some(items) = remaining_items {
                        if !items.is_empty() {
                            let _unused = self.data.add_frame(Frame::with_content(
                                id,
                                Content::InvolvedPeopleList(InvolvedPeopleList { items }),
                            ));
                        }
                    }
                }
                FrameId::DerivedValue(_, _) => (),
            }
        }
    }

    fn set_multiple<'a>(&'a mut self, key: &TagKey, values: &[Cow<'a, str>]) {
        if values.is_empty() {
            self.clear(key);
            return;
        }

        let frame = self.tag_key_to_frame(key);
        match frame {
            Some(FrameId::InvolvedPersonList(_)) => {
                unreachable!();
            }
            Some(FrameId::InvolvedPerson(id, involvement)) => {
                let items = self
                    .data
                    .get(id)
                    .and_then(|frame| frame.content().involved_people_list())
                    .into_iter()
                    .flat_map(|people_list| {
                        people_list
                            .items
                            .clone()
                            .into_iter()
                            .filter(|item| item.involvement.as_str() != involvement)
                    })
                    .chain(values.iter().map(|involvee| InvolvedPeopleListItem {
                        involvement: involvement.to_owned(),
                        involvee: involvee.to_string(),
                    }))
                    .collect::<Vec<_>>();
                let _unused = self.data.remove(id);
                if !items.is_empty() {
                    let _unused = self.data.add_frame(Frame::with_content(
                        id,
                        Content::InvolvedPeopleList(InvolvedPeopleList { items }),
                    ));
                }
            }
            _ => {
                self.set(key, values.join(" / ").into());
            }
        }
    }

    fn set(&mut self, key: &TagKey, value: Cow<'_, str>) {
        let frame = self.tag_key_to_frame(key);
        if let Some(frame) = frame {
            match frame {
                FrameId::Text(id) => {
                    self.data.set_text(id, value);
                }
                FrameId::CombinedText(id, CombinedTextPart::First) => {
                    let new_value = match self.get_combined_text_part(id, CombinedTextPart::Second)
                    {
                        Some(second_value) => Cow::from(format!("{value}/{second_value}")),
                        None => value,
                    };
                    self.data.set_text(id, new_value);
                }
                FrameId::CombinedText(id, CombinedTextPart::Second) => {
                    match self.get_combined_text_part(id, CombinedTextPart::First) {
                        Some(first_value) => {
                            let value = format!("{first_value}/{value}");
                            self.data.set_text(id, value);
                        }
                        None => {
                            let _frames = self.data.remove(id);
                        }
                    };
                }
                #[expect(unused_results)]
                FrameId::ExtendedText(description) => {
                    self.data.add_frame(ExtendedText {
                        description: description.to_string(),
                        value: value.into_owned(),
                    });
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
                FrameId::InvolvedPersonList(_) => {
                    unreachable!();
                }
                FrameId::InvolvedPerson(id, involvement) => {
                    let items = self
                        .data
                        .get(id)
                        .and_then(|frame| frame.content().involved_people_list())
                        .into_iter()
                        .flat_map(|people_list| {
                            people_list
                                .items
                                .clone()
                                .into_iter()
                                .filter(|item| item.involvement.as_str() != involvement)
                        })
                        .chain(iter::once(InvolvedPeopleListItem {
                            involvement: involvement.to_owned(),
                            involvee: value.to_string(),
                        }))
                        .collect::<Vec<_>>();
                    let _unused = self.data.remove(id);
                    if !items.is_empty() {
                        let _unused = self.data.add_frame(Frame::with_content(
                            id,
                            Content::InvolvedPeopleList(InvolvedPeopleList { items }),
                        ));
                    }
                }
                FrameId::DerivedValue(_, _) => (),
            }
        }
    }

    fn write(&mut self, path: &Path) -> crate::Result<()> {
        self.data.write_to_path(path, self.data.version())?;
        Ok(())
    }

    fn maybe_as_id3v2_mut(&mut self) -> Option<&mut ID3v2Tag> {
        Some(self)
    }

    fn performers(&self) -> Option<Vec<InvolvedPerson<'_>>> {
        self.tag_key_to_frame(&TagKey::Performers)
            .and_then(|frame_id| match frame_id {
                FrameId::InvolvedPersonList(id) => self.data.get(id),
                _ => None,
            })
            .and_then(|frame| frame.content().involved_people_list())
            .map(|people_list| {
                people_list
                    .items
                    .iter()
                    .filter(|item| match self.data.version() {
                        id3::Version::Id3v22 | id3::Version::Id3v23 => {
                            !IPLS_NON_PERFORMER_INVOLVEMENTS.contains(&item.involvement.as_str())
                        }
                        id3::Version::Id3v24 => true,
                    })
                    .map(|item| InvolvedPerson {
                        involvement: Cow::from(&item.involvement),
                        involvee: Cow::from(&item.involvee),
                    })
                    .collect()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag::{Tag, TagKey};
    use id3::Version;
    use paste::paste;
    use std::io::Cursor;

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
        let mut tag = ID3v2Tag::default();
        assert!(tag.get(&TagKey::Arranger).is_none());
        assert!(tag.get(&TagKey::Engineer).is_none());
        assert!(tag.get(&TagKey::DjMixer).is_none());
        assert!(tag.get(&TagKey::Mixer).is_none());
        assert!(tag.get(&TagKey::Producer).is_none());

        tag.set(&TagKey::Arranger, Cow::from("An awesome Arranger"));

        assert_eq!(
            tag.get(&TagKey::Arranger).as_deref(),
            Some("An awesome Arranger")
        );
        assert!(tag.get(&TagKey::Engineer).is_none());
        assert!(tag.get(&TagKey::DjMixer).is_none());
        assert!(tag.get(&TagKey::Mixer).is_none());
        assert!(tag.get(&TagKey::Producer).is_none());

        tag.set(&TagKey::Engineer, Cow::from("Mrs. Engineer"));

        assert_eq!(
            tag.get(&TagKey::Arranger).as_deref(),
            Some("An awesome Arranger")
        );
        assert_eq!(tag.get(&TagKey::Engineer).as_deref(), Some("Mrs. Engineer"));
        assert!(tag.get(&TagKey::DjMixer).is_none());
        assert!(tag.get(&TagKey::Mixer).is_none());
        assert!(tag.get(&TagKey::Producer).is_none());

        tag.set(&TagKey::DjMixer, Cow::from("Mr. DJ"));

        assert_eq!(
            tag.get(&TagKey::Arranger).as_deref(),
            Some("An awesome Arranger")
        );
        assert_eq!(tag.get(&TagKey::Engineer).as_deref(), Some("Mrs. Engineer"));
        assert_eq!(tag.get(&TagKey::DjMixer).as_deref(), Some("Mr. DJ"));
        assert!(tag.get(&TagKey::Mixer).is_none());
        assert!(tag.get(&TagKey::Producer).is_none());

        tag.set(&TagKey::Mixer, Cow::from("Miss Mixer"));

        assert_eq!(
            tag.get(&TagKey::Arranger).as_deref(),
            Some("An awesome Arranger")
        );
        assert_eq!(tag.get(&TagKey::Engineer).as_deref(), Some("Mrs. Engineer"));
        assert_eq!(tag.get(&TagKey::DjMixer).as_deref(), Some("Mr. DJ"));
        assert_eq!(tag.get(&TagKey::Mixer).as_deref(), Some("Miss Mixer"));
        assert!(tag.get(&TagKey::Producer).is_none());

        tag.set(&TagKey::Producer, Cow::from("Producer Dude"));

        assert_eq!(
            tag.get(&TagKey::Arranger).as_deref(),
            Some("An awesome Arranger")
        );
        assert_eq!(tag.get(&TagKey::Engineer).as_deref(), Some("Mrs. Engineer"));
        assert_eq!(tag.get(&TagKey::DjMixer).as_deref(), Some("Mr. DJ"));
        assert_eq!(tag.get(&TagKey::Mixer).as_deref(), Some("Miss Mixer"));
        assert_eq!(tag.get(&TagKey::Producer).as_deref(), Some("Producer Dude"));

        tag.clear(&TagKey::DjMixer);

        assert_eq!(
            tag.get(&TagKey::Arranger).as_deref(),
            Some("An awesome Arranger")
        );
        assert_eq!(tag.get(&TagKey::Engineer).as_deref(), Some("Mrs. Engineer"));
        assert!(tag.get(&TagKey::DjMixer).is_none());
        assert_eq!(tag.get(&TagKey::Mixer).as_deref(), Some("Miss Mixer"));
        assert_eq!(tag.get(&TagKey::Producer).as_deref(), Some("Producer Dude"));

        tag.clear(&TagKey::Arranger);

        assert!(tag.get(&TagKey::Arranger).is_none());
        assert_eq!(tag.get(&TagKey::Engineer).as_deref(), Some("Mrs. Engineer"));
        assert!(tag.get(&TagKey::DjMixer).is_none());
        assert_eq!(tag.get(&TagKey::Mixer).as_deref(), Some("Miss Mixer"));
        assert_eq!(tag.get(&TagKey::Producer).as_deref(), Some("Producer Dude"));

        tag.set_or_clear(&TagKey::Mixer, None);

        assert!(tag.get(&TagKey::Arranger).is_none());
        assert_eq!(tag.get(&TagKey::Engineer).as_deref(), Some("Mrs. Engineer"));
        assert!(tag.get(&TagKey::DjMixer).is_none());
        assert!(tag.get(&TagKey::Mixer).is_none());
        assert_eq!(tag.get(&TagKey::Producer).as_deref(), Some("Producer Dude"));
    }

    #[test]
    fn test_id3v23_utf16_read() {
        const MP3_DATA: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/media/picard-2.12.3/track-id3v23-utf16.mp3"
        ));
        let cursor = Cursor::new(MP3_DATA);
        let tag = ID3v2Tag {
            data: id3::Tag::read_from2(cursor).unwrap(),
        };
        assert_eq!(tag.tag_type(), TagType::ID3v23);
        assert_eq!(
            tag.get(&TagKey::TrackTitle).as_deref(),
            Some("But Not for Me")
        );
        assert_eq!(
            tag.get(&TagKey::Artist).as_deref(),
            Some("The Ahmad Jamal Trio")
        );
        assert_eq!(
            tag.get(&TagKey::Album).as_deref(),
            Some("Ahmad Jamal at the Pershing: But Not for Me")
        );
        assert_eq!(tag.get(&TagKey::TrackNumber).as_deref(), Some("1"));
        assert_eq!(tag.get(&TagKey::ReleaseYear).as_deref(), Some("1958"));
        assert_eq!(
            tag.get(&TagKey::AlbumArtist).as_deref(),
            Some("The Ahmad Jamal Trio")
        );
        assert_eq!(
            tag.get(&TagKey::AlbumArtistSortOrder).as_deref(),
            Some("Jamal, Ahmad, Trio, The")
        );
        assert_eq!(
            tag.get(&TagKey::Artists).as_deref(),
            Some("The Ahmad Jamal Trio")
        );
        assert_eq!(
            tag.get(&TagKey::CatalogNumber).as_deref(),
            Some("LP-628/LPS-628")
        );
        assert_eq!(
            tag.get(&TagKey::Composer).as_deref(),
            Some("George Gershwin")
        );
        assert_eq!(
            tag.get(&TagKey::ComposerSortOrder).as_deref(),
            Some("Gershwin, George")
        );
        assert_eq!(tag.get(&TagKey::DiscNumber).as_deref(), Some("1"));
        assert_eq!(tag.get(&TagKey::Language).as_deref(), Some("zxx"));
        assert_eq!(tag.get(&TagKey::Media).as_deref(), Some("12\" Vinyl"));
        assert_eq!(
            tag.get(&TagKey::MusicBrainzArtistId).as_deref(),
            Some("9e7ca87b-4e3d-4d14-90f1-a74acb645fe2")
        );
        assert_eq!(
            tag.get(&TagKey::MusicBrainzRecordingId).as_deref(),
            Some("9d444787-3f25-4c16-9261-597b9ab021cc")
        );
        assert_eq!(
            tag.get(&TagKey::MusicBrainzReleaseArtistId).as_deref(),
            Some("9e7ca87b-4e3d-4d14-90f1-a74acb645fe2")
        );
        assert_eq!(
            tag.get(&TagKey::MusicBrainzReleaseGroupId).as_deref(),
            Some("0a8e97fd-457c-30bc-938a-2fba79cb04e7")
        );
        assert_eq!(
            tag.get(&TagKey::MusicBrainzReleaseId).as_deref(),
            Some("0008f765-032b-46cd-ab69-2220edab1837")
        );
        assert_eq!(
            tag.get(&TagKey::MusicBrainzTrackId).as_deref(),
            Some("cc9757af-8427-386e-aced-75b800feed77")
        );
        assert_eq!(
            tag.get(&TagKey::MusicBrainzWorkId).as_deref(),
            Some("f53d7dd0-fdbd-3901-adf8-9b1ab3121e9e")
        );
        assert_eq!(
            tag.get(&TagKey::OriginalReleaseDate).as_deref(),
            Some("1958")
        );
        assert_eq!(
            tag.performers(),
            Some(vec![
                InvolvedPerson {
                    involvement: "double bass".into(),
                    involvee: "Israel Crosby".into(),
                },
                InvolvedPerson {
                    involvement: "drums (drum set)".into(),
                    involvee: "Vernell Fournier".into(),
                },
                InvolvedPerson {
                    involvement: "piano".into(),
                    involvee: "Ahmad Jamal".into(),
                }
            ])
        );
        assert_eq!(tag.get(&TagKey::Producer).as_deref(), Some("Dave Usher"));
        assert_eq!(tag.get(&TagKey::RecordLabel).as_deref(), Some("Argo"));
        assert_eq!(tag.get(&TagKey::ReleaseCountry).as_deref(), Some("US"));
        assert_eq!(tag.get(&TagKey::ReleaseStatus).as_deref(), Some("official"));
        assert_eq!(tag.get(&TagKey::ReleaseType).as_deref(), Some("album/live"));
        assert_eq!(tag.get(&TagKey::Script).as_deref(), Some("Latn"));
        assert_eq!(tag.get(&TagKey::TotalDiscs).as_deref(), Some("1"));
        assert_eq!(tag.get(&TagKey::TotalTracks).as_deref(), Some("8"));
        assert_eq!(
            tag.get(&TagKey::WorkTitle).as_deref(),
            Some("But Not for Me")
        );
    }

    macro_rules! add_tests_with_id3_version {
        ($tagkey:expr, $version:expr, $fnsuffix:ident) => {
            paste! {
                #[test]
                fn [<test_get_set_ $fnsuffix>]() {
                    let mut tag = ID3v2Tag::with_version($version);
                    assert!(tag.get($tagkey).is_none());

                    tag.set($tagkey, Cow::from("Example Value"));
                    assert_eq!(tag.get($tagkey).as_deref(), Some("Example Value"));
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

    macro_rules! add_tests_with_id3_version_combinedtext {
        ($tagkey1:expr, $tagkey2:expr, $version:expr, $fnsuffix:ident) => {
            paste! {
                #[test]
                fn [<test_get_set_ $fnsuffix>]() {
                    let mut tag = ID3v2Tag::with_version($version);
                    assert!(tag.get($tagkey1).is_none());
                    assert!(tag.get($tagkey2).is_none());

                    tag.set($tagkey1, Cow::from("Example Value 1"));
                    assert_eq!(tag.get($tagkey1).as_deref(), Some("Example Value 1"));
                    assert!(tag.get($tagkey2).is_none());

                    tag.set($tagkey2, Cow::from("Example Value 2"));
                    assert_eq!(tag.get($tagkey1).as_deref(), Some("Example Value 1"));
                    assert_eq!(tag.get($tagkey2).as_deref(), Some("Example Value 2"));

                    tag.set($tagkey1, Cow::from("Example Value 3"));
                    assert_eq!(tag.get($tagkey1).as_deref(), Some("Example Value 3"));
                    assert_eq!(tag.get($tagkey2).as_deref(), Some("Example Value 2"));
                }

                #[test]
                fn [<test_clear_ $fnsuffix>]() {
                    let mut tag = ID3v2Tag::with_version($version);
                    assert!(tag.get($tagkey1).is_none());
                    assert!(tag.get($tagkey2).is_none());

                    tag.set($tagkey1, Cow::from("Example Value 1"));
                    tag.set($tagkey2, Cow::from("Example Value 2"));
                    assert!(tag.get($tagkey1).is_some());
                    assert!(tag.get($tagkey2).is_some());

                    tag.clear($tagkey2);
                    assert!(tag.get($tagkey1).is_some());
                    assert!(tag.get($tagkey2).is_none());

                    tag.clear($tagkey1);
                    assert!(tag.get($tagkey1).is_none());
                    assert!(tag.get($tagkey2).is_none());

                    tag.set($tagkey1, Cow::from("Example Value 1"));
                    tag.set($tagkey2, Cow::from("Example Value 2"));
                    assert!(tag.get($tagkey1).is_some());
                    assert!(tag.get($tagkey2).is_some());

                    tag.clear($tagkey1);
                    assert!(tag.get($tagkey1).is_none());
                    assert!(tag.get($tagkey2).is_none());
                }

                #[test]
                fn [<test_set_or_clear_ $fnsuffix>]() {
                    let mut tag = ID3v2Tag::with_version($version);
                    assert!(tag.get($tagkey1).is_none());

                    tag.set_or_clear($tagkey1, Some(Cow::from("Example Value")));
                    assert!(tag.get($tagkey1).is_some());

                    tag.set_or_clear($tagkey2, Some(Cow::from("Other Value")));
                    assert!(tag.get($tagkey1).is_some());
                    assert!(tag.get($tagkey2).is_some());

                    tag.set_or_clear($tagkey2, None);
                    assert!(tag.get($tagkey1).is_some());
                    assert!(tag.get($tagkey2).is_none());
                }
            }
        };
    }
    macro_rules! add_tests_with_id3_versions_all_combinedtext {
        ($tagkey1:pat_param, $tagkey2:pat_param, $fnsuffix:ident) => {
            paste! {
                add_tests_with_id3_version_combinedtext!($tagkey1, $tagkey2, Version::Id3v22, [< $fnsuffix _id3v22>]);
                add_tests_with_id3_version_combinedtext!($tagkey1, $tagkey2, Version::Id3v23, [< $fnsuffix _id3v23>]);
                add_tests_with_id3_version_combinedtext!($tagkey1, $tagkey2, Version::Id3v24, [< $fnsuffix _id3v24>]);
            }
        };
    }

    add_tests_with_id3_versions_all!(&TagKey::AcoustId, acoustid);
    add_tests_with_id3_versions_all!(&TagKey::AcoustIdFingerprint, acoustidfingerprint);
    add_tests_with_id3_versions_all!(&TagKey::Album, album);
    add_tests_with_id3_versions_all!(&TagKey::AlbumArtist, albumartist);
    add_tests_with_id3_versions_all!(&TagKey::AlbumArtistSortOrder, albumartistsortorder);
    add_tests_with_id3_versions_all!(&TagKey::AlbumSortOrder, albumsortorder);
    add_tests_with_id3_versions_all!(&TagKey::Artist, artist);
    add_tests_with_id3_versions_all!(&TagKey::ArtistSortOrder, artistsortorder);
    add_tests_with_id3_versions_all!(&TagKey::Artists, artists);
    add_tests_with_id3_versions_all!(&TagKey::Asin, asin);
    add_tests_with_id3_versions_all!(&TagKey::Barcode, barcode);
    add_tests_with_id3_versions_all!(&TagKey::Bpm, bpm);
    add_tests_with_id3_versions_all!(&TagKey::CatalogNumber, catalognumber);
    add_tests_with_id3_versions_all!(&TagKey::Comment, comment);
    add_tests_with_id3_versions_all!(&TagKey::Compilation, compilation);
    add_tests_with_id3_versions_all!(&TagKey::Composer, composer);
    add_tests_with_id3_versions_all!(&TagKey::ComposerSortOrder, composersortorder);
    add_tests_with_id3_versions_all!(&TagKey::Conductor, conductor);
    add_tests_with_id3_versions_all!(&TagKey::Copyright, copyright);
    add_tests_with_id3_versions_all!(&TagKey::Director, director);
    add_tests_with_id3_versions_all!(&TagKey::DiscNumber, discnumber);
    add_tests_with_id3_version!(&TagKey::DiscSubtitle, Version::Id3v24, discsubtitle_id3v24);
    add_tests_with_id3_versions_all!(&TagKey::EncodedBy, encodedby);
    add_tests_with_id3_versions_all!(&TagKey::EncoderSettings, encodersettings);
    //add_tests_with_id3_versions_all!(&TagKey::GaplessPlayback, gaplessplayback);
    add_tests_with_id3_versions_all!(&TagKey::Genre, genre);
    add_tests_with_id3_versions_all!(&TagKey::Grouping, grouping);
    add_tests_with_id3_versions_all!(&TagKey::InitialKey, initialkey);
    add_tests_with_id3_versions_all!(&TagKey::Isrc, isrc);
    add_tests_with_id3_versions_all!(&TagKey::Language, language);
    //add_tests_with_id3_versions_all!(&TagKey::License, license);
    add_tests_with_id3_versions_all!(&TagKey::Lyricist, lyricist);
    //add_tests_with_id3_versions_all!(&TagKey::Lyrics, lyrics);
    add_tests_with_id3_versions_all!(&TagKey::Media, media);
    add_tests_with_id3_version!(&TagKey::Mood, Version::Id3v24, mood_id3v24);
    add_tests_with_id3_versions_all!(&TagKey::Movement, movement);
    add_tests_with_id3_versions_all!(&TagKey::MovementCount, movementcount);
    add_tests_with_id3_versions_all!(&TagKey::MovementNumber, movementnumber);
    add_tests_with_id3_versions_all!(&TagKey::MusicBrainzArtistId, musicbrainzartistid);
    add_tests_with_id3_versions_all!(&TagKey::MusicBrainzDiscId, musicbrainzdiscid);
    add_tests_with_id3_versions_all!(
        &TagKey::MusicBrainzOriginalArtistId,
        musicbrainzoriginalartistid
    );
    add_tests_with_id3_versions_all!(
        &TagKey::MusicBrainzOriginalReleaseId,
        musicbrainzoriginalreleaseid
    );
    add_tests_with_id3_versions_all!(&TagKey::MusicBrainzRecordingId, musicbrainzrecordingid);
    add_tests_with_id3_versions_all!(
        &TagKey::MusicBrainzReleaseArtistId,
        musicbrainzreleaseartistid
    );
    add_tests_with_id3_versions_all!(
        &TagKey::MusicBrainzReleaseGroupId,
        musicbrainzreleasegroupid
    );
    add_tests_with_id3_versions_all!(&TagKey::MusicBrainzReleaseId, musicbrainzreleaseid);
    add_tests_with_id3_versions_all!(&TagKey::MusicBrainzTrackId, musicbrainztrackid);
    add_tests_with_id3_versions_all!(&TagKey::MusicBrainzTrmId, musicbrainztrmid);
    add_tests_with_id3_versions_all!(&TagKey::MusicBrainzWorkId, musicbrainzworkid);
    add_tests_with_id3_versions_all!(&TagKey::MusicIpFingerprint, musicipfingerprint);
    add_tests_with_id3_versions_all!(&TagKey::MusicIpPuid, musicippuid);
    add_tests_with_id3_versions_all!(&TagKey::OriginalAlbum, originalalbum);
    add_tests_with_id3_versions_all!(&TagKey::OriginalArtist, originalartist);
    add_tests_with_id3_versions_all!(&TagKey::OriginalFilename, originalfilename);
    add_tests_with_id3_version!(
        &TagKey::OriginalReleaseDate,
        Version::Id3v23,
        originalreleasedate_id3v23
    );
    add_tests_with_id3_version!(
        &TagKey::OriginalReleaseDate,
        Version::Id3v24,
        originalreleasedate_id3v24
    );
    //add_tests_with_id3_versions_all!(&TagKey::OriginalReleaseYear, originalreleaseyear);
    //add_tests_with_id3_versions_all!(&TagKey::Performer, performers);
    //add_tests_with_id3_versions_all!(&TagKey::Podcast, podcast);
    //add_tests_with_id3_versions_all!(&TagKey::PodcastUrl, podcasturl);
    add_tests_with_id3_versions_all!(&TagKey::Rating, rating);
    add_tests_with_id3_versions_all!(&TagKey::RecordLabel, recordlabel);
    add_tests_with_id3_versions_all!(&TagKey::ReleaseCountry, releasecountry);
    add_tests_with_id3_version!(&TagKey::ReleaseDate, Version::Id3v23, releasedate_id3v23);
    add_tests_with_id3_version!(&TagKey::ReleaseDate, Version::Id3v24, releasedate_id3v24);
    add_tests_with_id3_version!(&TagKey::ReleaseYear, Version::Id3v23, releaseyear_id3v23);
    add_tests_with_id3_versions_all!(&TagKey::ReleaseStatus, releasestatus);
    add_tests_with_id3_versions_all!(&TagKey::ReleaseType, releasetype);
    add_tests_with_id3_versions_all!(&TagKey::Remixer, remixer);
    add_tests_with_id3_versions_all!(&TagKey::ReplayGainAlbumGain, replaygainalbumgain);
    add_tests_with_id3_versions_all!(&TagKey::ReplayGainAlbumPeak, replaygainalbumpeak);
    add_tests_with_id3_versions_all!(&TagKey::ReplayGainAlbumRange, replaygainalbumrange);
    add_tests_with_id3_versions_all!(
        &TagKey::ReplayGainReferenceLoudness,
        replaygainreferenceloudness
    );
    add_tests_with_id3_versions_all!(&TagKey::ReplayGainTrackGain, replaygaintrackgain);
    add_tests_with_id3_versions_all!(&TagKey::ReplayGainTrackPeak, replaygaintrackpeak);
    add_tests_with_id3_versions_all!(&TagKey::ReplayGainTrackRange, replaygaintrackrange);
    add_tests_with_id3_versions_all!(&TagKey::Script, script);
    //add_tests_with_id3_versions_all!(&TagKey::ShowName, showname);
    //add_tests_with_id3_versions_all!(&TagKey::ShowNameSortOrder, shownamesortorder);
    add_tests_with_id3_versions_all!(&TagKey::ShowMovement, showmovement);
    add_tests_with_id3_versions_all!(&TagKey::Subtitle, subtitle);
    add_tests_with_id3_versions_all_combinedtext!(
        &TagKey::DiscNumber,
        &TagKey::TotalDiscs,
        totaldiscs
    );
    add_tests_with_id3_versions_all_combinedtext!(
        &TagKey::TrackNumber,
        &TagKey::TotalTracks,
        totaltracks
    );
    add_tests_with_id3_versions_all!(&TagKey::TrackNumber, tracknumber);
    add_tests_with_id3_versions_all!(&TagKey::TrackTitle, tracktitle);
    add_tests_with_id3_versions_all!(&TagKey::TrackTitleSortOrder, tracktitlesortorder);
    add_tests_with_id3_versions_all!(&TagKey::ArtistWebsite, artistwebsite);
    add_tests_with_id3_versions_all!(&TagKey::WorkTitle, worktitle);
    add_tests_with_id3_versions_all!(&TagKey::Writer, writer);
}
