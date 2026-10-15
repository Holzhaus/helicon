// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Support for M4A/MP4 ("iTunes-style") tags.
//!
//! This module mirrors the structure of the `flac` and `id3` modules: a `tag_key_to_item`
//! function maps a [`TagKey`] onto the underlying storage primitive, and the [`Tag`] trait is
//! implemented in terms of that mapping. The mapping follows the "iTunes MP4" column of
//! `assets/mapping.tsv`.
//!
//! Storage is delegated to the [`mp4ameta`](https://docs.rs/mp4ameta) crate, which models MP4
//! metadata as "atoms" identified by an [`Ident`](mp4ameta::Ident):
//!
//! * A four-byte "fourcc" atom, e.g. `©nam` for the track title.
//! * A freeform `----` atom, identified by a `mean`/`name` pair, e.g.
//!   `----:com.apple.iTunes:MusicBrainz Artist Id`. Helicon only ever uses freeform atoms with
//!   the `mean` set to `com.apple.iTunes`, matching what Picard/iTunes do.

use crate::tag::{Tag, TagKey, TagType};
use crate::track::InvolvedPerson;
use mp4ameta::{
    ident::{self, DataIdent},
    Data, Fourcc, Tag as Mp4TagData,
};
use std::borrow::Cow;
use std::path::Path;

/// The storage primitive used for a [`TagKey`].
#[derive(Debug, Clone)]
enum TagItem {
    /// Plain UTF-8 text.
    Text(DataIdent),
    /// A boolean flag stored as a single byte.
    Bool(Fourcc),
    /// A big-endian 16-bit integer.
    Int(Fourcc),
    /// The track number or total number of tracks (both are stored in `trkn`).
    Track(TuplePart),
    /// The disc number or total number of discs (both are stored in `disk`).
    Disc(TuplePart),
}

/// Selects one half of a combined "number / total" atom.
#[derive(Debug, Clone, Copy)]
enum TuplePart {
    /// The number (first half).
    Number,
    /// The total (second half).
    Total,
}

/// A freeform `com.apple.iTunes` text atom.
#[expect(clippy::unnecessary_wraps)]
fn freeform(name: &'static str) -> Option<TagItem> {
    Some(TagItem::Text(DataIdent::freeform(
        ident::APPLE_ITUNES_MEAN,
        name,
    )))
}

/// A regular fourcc text atom.
#[expect(clippy::unnecessary_wraps)]
fn text(fourcc: Fourcc) -> Option<TagItem> {
    Some(TagItem::Text(DataIdent::from(fourcc)))
}

/// Parse a big-endian integer of up to two bytes.
fn parse_be_u16(bytes: &[u8]) -> Option<u16> {
    match *bytes {
        [a] => Some(u16::from(a)),
        [a, b, ..] => Some(u16::from_be_bytes([a, b])),
        [] => None,
    }
}

/// M4A/MP4 tag.
pub struct Mp4Tag {
    /// The underlying tag data.
    data: Mp4TagData,
}

impl Mp4Tag {
    #[cfg(test)]
    pub fn new() -> Self {
        Mp4Tag {
            data: Mp4TagData::default(),
        }
    }

    /// Read the M4A tag from the path.
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        let data = Mp4TagData::read_from_path(path)?;
        Ok(Mp4Tag { data })
    }

    /// Get the storage primitive for a tag key.
    ///
    /// Returns `None` wherever `assets/mapping.tsv` lists `n/a` in the "iTunes MP4" column.
    fn tag_key_to_item(key: &TagKey) -> Option<TagItem> {
        #[expect(clippy::match_same_arms)]
        match key {
            TagKey::AcoustId => freeform("Acoustid Id"),
            TagKey::AcoustIdFingerprint => freeform("Acoustid Fingerprint"),
            TagKey::Album => text(ident::ALBUM),
            TagKey::AlbumArtist => text(ident::ALBUM_ARTIST),
            TagKey::AlbumArtistSortOrder => text(ident::ALBUM_ARTIST_SORT_ORDER),
            TagKey::AlbumSortOrder => text(ident::ALBUM_SORT_ORDER),
            TagKey::Arranger => None,
            TagKey::Artist => text(ident::ARTIST),
            TagKey::ArtistSortOrder => text(ident::ARTIST_SORT_ORDER),
            TagKey::Artists => freeform("ARTISTS"),
            TagKey::ArtistWebsite => None,
            TagKey::Asin => freeform("ASIN"),
            TagKey::Barcode => freeform("BARCODE"),
            TagKey::Bpm => Some(TagItem::Int(ident::BPM)),
            TagKey::CatalogNumber => freeform("CATALOGNUMBER"),
            TagKey::Comment => text(ident::COMMENT),
            TagKey::Compilation => Some(TagItem::Bool(ident::COMPILATION)),
            TagKey::Composer => text(ident::COMPOSER),
            TagKey::ComposerSortOrder => text(ident::COMPOSER_SORT_ORDER),
            TagKey::Conductor => freeform("CONDUCTOR"),
            TagKey::Copyright => text(ident::COPYRIGHT),
            TagKey::Director => text(Fourcc(*b"\xa9dir")),
            TagKey::DiscNumber => Some(TagItem::Disc(TuplePart::Number)),
            TagKey::DiscSubtitle => freeform("DISCSUBTITLE"),
            TagKey::DjMixer => freeform("DJMIXER"),
            TagKey::EncodedBy => text(ident::ENCODER),
            TagKey::EncoderSettings => None,
            TagKey::Engineer => freeform("ENGINEER"),
            TagKey::GaplessPlayback => Some(TagItem::Bool(ident::GAPLESS_PLAYBACK)),
            // Free-text genre; `gnre` is the legacy numeric ID3v1 genre and isn't used here.
            TagKey::Genre => text(ident::CUSTOM_GENRE),
            TagKey::Grouping => text(ident::GROUPING),
            TagKey::InitialKey => freeform("initialkey"),
            TagKey::Isrc => freeform("ISRC"),
            TagKey::Language => freeform("LANGUAGE"),
            TagKey::License => freeform("LICENSE"),
            TagKey::Lyricist => freeform("LYRICIST"),
            TagKey::Lyrics => text(ident::LYRICS),
            TagKey::Media => freeform("MEDIA"),
            TagKey::Mixer => freeform("MIXER"),
            TagKey::Mood => freeform("MOOD"),
            TagKey::Movement => text(ident::MOVEMENT),
            TagKey::MovementCount => Some(TagItem::Int(ident::MOVEMENT_COUNT)),
            TagKey::MovementNumber => Some(TagItem::Int(ident::MOVEMENT_INDEX)),
            TagKey::MusicBrainzArtistId => freeform("MusicBrainz Artist Id"),
            TagKey::MusicBrainzDiscId => freeform("MusicBrainz Disc Id"),
            TagKey::MusicBrainzOriginalArtistId => freeform("MusicBrainz Original Artist Id"),
            TagKey::MusicBrainzOriginalReleaseId => freeform("MusicBrainz Original Album Id"),
            TagKey::MusicBrainzRecordingId => freeform("MusicBrainz Track Id"),
            TagKey::MusicBrainzReleaseArtistId => freeform("MusicBrainz Album Artist Id"),
            TagKey::MusicBrainzReleaseGroupId => freeform("MusicBrainz Release Group Id"),
            TagKey::MusicBrainzReleaseId => freeform("MusicBrainz Album Id"),
            TagKey::MusicBrainzTrackId => freeform("MusicBrainz Release Track Id"),
            TagKey::MusicBrainzTrmId => freeform("MusicBrainz TRM Id"),
            TagKey::MusicBrainzWorkId => freeform("MusicBrainz Work Id"),
            TagKey::MusicIpFingerprint => freeform("fingerprint"),
            TagKey::MusicIpPuid => freeform("MusicIP PUID"),
            TagKey::OriginalAlbum => None,
            TagKey::OriginalArtist => None,
            TagKey::OriginalFilename => None,
            TagKey::OriginalReleaseDate => None,
            TagKey::OriginalReleaseYear => None,
            TagKey::Performer(_) => None,
            TagKey::Performers => None,
            TagKey::Podcast => Some(TagItem::Bool(ident::PODCAST)),
            TagKey::PodcastUrl => text(ident::PODCAST_URL),
            TagKey::Producer => freeform("PRODUCER"),
            TagKey::Rating => None,
            TagKey::RecordLabel => freeform("LABEL"),
            TagKey::ReleaseCountry => freeform("MusicBrainz Album Release Country"),
            TagKey::ReleaseDate => None,
            TagKey::ReleaseYear => None,
            TagKey::ReleaseStatus => freeform("MusicBrainz Album Status"),
            TagKey::ReleaseType => freeform("MusicBrainz Album Type"),
            TagKey::Remixer => freeform("REMIXER"),
            TagKey::ReplayGainAlbumGain => freeform("REPLAYGAIN_ALBUM_GAIN"),
            TagKey::ReplayGainAlbumPeak => freeform("REPLAYGAIN_ALBUM_PEAK"),
            TagKey::ReplayGainAlbumRange => freeform("REPLAYGAIN_ALBUM_RANGE"),
            TagKey::ReplayGainReferenceLoudness => freeform("REPLAYGAIN_REFERENCE_LOUDNESS"),
            TagKey::ReplayGainTrackGain => freeform("REPLAYGAIN_TRACK_GAIN"),
            TagKey::ReplayGainTrackPeak => freeform("REPLAYGAIN_TRACK_PEAK"),
            TagKey::ReplayGainTrackRange => freeform("REPLAYGAIN_TRACK_RANGE"),
            TagKey::Script => freeform("SCRIPT"),
            TagKey::ShowMovement => Some(TagItem::Bool(ident::SHOW_MOVEMENT)),
            TagKey::ShowName => text(ident::TV_SHOW_NAME),
            TagKey::ShowNameSortOrder => text(ident::TV_SHOW_NAME_SORT_ORDER),
            TagKey::Subtitle => freeform("SUBTITLE"),
            TagKey::TotalDiscs => Some(TagItem::Disc(TuplePart::Total)),
            TagKey::TotalTracks => Some(TagItem::Track(TuplePart::Total)),
            TagKey::TrackNumber => Some(TagItem::Track(TuplePart::Number)),
            TagKey::TrackTitle => text(ident::TITLE),
            TagKey::TrackTitleSortOrder => text(ident::TITLE_SORT_ORDER),
            TagKey::WorkTitle => text(ident::WORK),
            TagKey::Writer => None,
        }
    }
}

impl Tag for Mp4Tag {
    fn tag_type(&self) -> TagType {
        TagType::MP4
    }

    fn get<'a>(&'a self, key: &'a TagKey) -> Option<Cow<'a, str>> {
        match Self::tag_key_to_item(key)? {
            TagItem::Text(ident) => self.data.strings_of(&ident).next().map(Cow::from),
            TagItem::Bool(fourcc) => {
                let set = self
                    .data
                    .bytes_of(&fourcc)
                    .next()
                    .and_then(|bytes| bytes.first())
                    .is_some_and(|&byte| byte == 1);
                Some(Cow::from(if set { "1" } else { "0" }))
            }
            TagItem::Int(fourcc) => self
                .data
                .bytes_of(&fourcc)
                .next()
                .and_then(parse_be_u16)
                .map(|value| Cow::from(value.to_string())),
            TagItem::Track(part) => match part {
                TuplePart::Number => self.data.track_number(),
                TuplePart::Total => self.data.total_tracks(),
            }
            .map(|value| Cow::from(value.to_string())),
            TagItem::Disc(part) => match part {
                TuplePart::Number => self.data.disc_number(),
                TuplePart::Total => self.data.total_discs(),
            }
            .map(|value| Cow::from(value.to_string())),
        }
    }

    fn set(&mut self, key: &TagKey, value: Cow<'_, str>) {
        self.set_multiple(key, &[value]);
    }

    fn set_multiple<'a>(&'a mut self, key: &TagKey, values: &[Cow<'a, str>]) {
        let Some(item) = Self::tag_key_to_item(key) else {
            return;
        };

        if values.is_empty() {
            self.clear(key);
            return;
        }

        match item {
            TagItem::Text(ident) => {
                self.data.set_all_data(
                    ident,
                    values.iter().map(|value| Data::Utf8(value.to_string())),
                );
            }
            TagItem::Bool(fourcc) => {
                let flag = u8::from(values[0].trim() == "1");
                self.data.set_data(fourcc, Data::BeSigned(vec![flag]));
            }
            TagItem::Int(fourcc) => match values[0].trim().parse::<u16>() {
                Ok(number) => self
                    .data
                    .set_data(fourcc, Data::BeSigned(number.to_be_bytes().to_vec())),
                Err(err) => log::warn!("Cannot write {:?} as integer to MP4 tag: {err}", values[0]),
            },
            TagItem::Track(part) | TagItem::Disc(part) => {
                let Ok(number) = values[0].trim().parse::<u16>() else {
                    log::warn!("Cannot write {:?} as integer to MP4 tag", values[0]);
                    return;
                };
                let is_track = matches!(Self::tag_key_to_item(key), Some(TagItem::Track(_)));
                match (is_track, part) {
                    (true, TuplePart::Number) => self.data.set_track_number(number),
                    (true, TuplePart::Total) => self.data.set_total_tracks(number),
                    (false, TuplePart::Number) => self.data.set_disc_number(number),
                    (false, TuplePart::Total) => self.data.set_total_discs(number),
                }
            }
        }
    }

    fn clear(&mut self, key: &TagKey) {
        match Self::tag_key_to_item(key) {
            Some(TagItem::Text(ident)) => self.data.remove_data_of(&ident),
            Some(TagItem::Bool(fourcc) | TagItem::Int(fourcc)) => {
                self.data.remove_data_of(&fourcc);
            }
            Some(TagItem::Track(part)) => {
                match part {
                    TuplePart::Number => self.data.set_track_number(0),
                    TuplePart::Total => self.data.set_total_tracks(0),
                }
                if self.data.track_number().is_none() && self.data.total_tracks().is_none() {
                    self.data.remove_track();
                }
            }
            Some(TagItem::Disc(part)) => {
                match part {
                    TuplePart::Number => self.data.set_disc_number(0),
                    TuplePart::Total => self.data.set_total_discs(0),
                }
                if self.data.disc_number().is_none() && self.data.total_discs().is_none() {
                    self.data.remove_disc();
                }
            }
            None => (),
        }
    }

    fn write(&mut self, path: &Path) -> crate::Result<()> {
        self.data.write_to_path(path)?;
        Ok(())
    }

    fn performers(&self) -> Option<Vec<InvolvedPerson<'_>>> {
        // MP4/iTunes atoms have no standard equivalent of ID3's IPLS/TMCL or Vorbis's PERFORMER
        // list, and assets/mapping.tsv lists "n/a" for performers, so there is nothing to parse.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag::{Tag, TagKey};
    use paste::paste;

    #[test]
    fn test_tag_type() {
        let tag = Mp4Tag::new();
        assert_eq!(tag.tag_type(), TagType::MP4);
    }

    macro_rules! add_tests {
        ($tagkey:expr, $fnsuffix:ident) => {
            paste! {
                #[test]
                fn [<test_get_set_ $fnsuffix>]() {
                    let mut tag = Mp4Tag::new();
                    assert!(tag.get($tagkey).is_none());

                    tag.set($tagkey, Cow::from("Example Value"));
                    assert_eq!(tag.get($tagkey).as_deref(), Some("Example Value"));
                }

                #[test]
                fn [<test_clear_ $fnsuffix>]() {
                    let mut tag = Mp4Tag::new();
                    assert!(tag.get($tagkey).is_none());

                    tag.set($tagkey, Cow::from("Example Value"));
                    assert!(tag.get($tagkey).is_some());

                    tag.clear($tagkey);
                    assert!(tag.get($tagkey).is_none());
                }

                #[test]
                fn [<test_set_or_clear_ $fnsuffix>]() {
                    let mut tag = Mp4Tag::new();
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

    add_tests!(&TagKey::AcoustId, acoustid);
    add_tests!(&TagKey::AcoustIdFingerprint, acoustidfingerprint);
    add_tests!(&TagKey::Album, album);
    add_tests!(&TagKey::AlbumArtist, albumartist);
    add_tests!(&TagKey::AlbumArtistSortOrder, albumartistsortorder);
    add_tests!(&TagKey::AlbumSortOrder, albumsortorder);
    add_tests!(&TagKey::Artist, artist);
    add_tests!(&TagKey::ArtistSortOrder, artistsortorder);
    add_tests!(&TagKey::Artists, artists);
    add_tests!(&TagKey::Asin, asin);
    add_tests!(&TagKey::Barcode, barcode);
    add_tests!(&TagKey::CatalogNumber, catalognumber);
    add_tests!(&TagKey::Comment, comment);
    add_tests!(&TagKey::Composer, composer);
    add_tests!(&TagKey::ComposerSortOrder, composersortorder);
    add_tests!(&TagKey::Conductor, conductor);
    add_tests!(&TagKey::Copyright, copyright);
    add_tests!(&TagKey::Director, director);
    add_tests!(&TagKey::DiscSubtitle, discsubtitle);
    add_tests!(&TagKey::EncodedBy, encodedby);
    add_tests!(&TagKey::Engineer, engineer);
    add_tests!(&TagKey::Genre, genre);
    add_tests!(&TagKey::Grouping, grouping);
    add_tests!(&TagKey::InitialKey, initialkey);
    add_tests!(&TagKey::Isrc, isrc);
    add_tests!(&TagKey::Language, language);
    add_tests!(&TagKey::License, license);
    add_tests!(&TagKey::Lyricist, lyricist);
    add_tests!(&TagKey::Lyrics, lyrics);
    add_tests!(&TagKey::Media, media);
    add_tests!(&TagKey::DjMixer, djmixer);
    add_tests!(&TagKey::Mixer, mixer);
    add_tests!(&TagKey::Mood, mood);
    add_tests!(&TagKey::Movement, movement);
    add_tests!(&TagKey::MusicBrainzArtistId, musicbrainzartistid);
    add_tests!(&TagKey::MusicBrainzDiscId, musicbrainzdiscid);
    add_tests!(
        &TagKey::MusicBrainzOriginalArtistId,
        musicbrainzoriginalartistid
    );
    add_tests!(
        &TagKey::MusicBrainzOriginalReleaseId,
        musicbrainzoriginalreleaseid
    );
    add_tests!(&TagKey::MusicBrainzRecordingId, musicbrainzrecordingid);
    add_tests!(
        &TagKey::MusicBrainzReleaseArtistId,
        musicbrainzreleaseartistid
    );
    add_tests!(
        &TagKey::MusicBrainzReleaseGroupId,
        musicbrainzreleasegroupid
    );
    add_tests!(&TagKey::MusicBrainzReleaseId, musicbrainzreleaseid);
    add_tests!(&TagKey::MusicBrainzTrackId, musicbrainztrackid);
    add_tests!(&TagKey::MusicBrainzTrmId, musicbrainztrmid);
    add_tests!(&TagKey::MusicBrainzWorkId, musicbrainzworkid);
    add_tests!(&TagKey::MusicIpFingerprint, musicipfingerprint);
    add_tests!(&TagKey::MusicIpPuid, musicippuid);
    add_tests!(&TagKey::Producer, producer);
    add_tests!(&TagKey::RecordLabel, recordlabel);
    add_tests!(&TagKey::ReleaseCountry, releasecountry);
    add_tests!(&TagKey::ReleaseStatus, releasestatus);
    add_tests!(&TagKey::ReleaseType, releasetype);
    add_tests!(&TagKey::Remixer, remixer);
    add_tests!(&TagKey::ReplayGainAlbumGain, replaygainalbumgain);
    add_tests!(&TagKey::ReplayGainAlbumPeak, replaygainalbumpeak);
    add_tests!(&TagKey::ReplayGainAlbumRange, replaygainalbumrange);
    add_tests!(
        &TagKey::ReplayGainReferenceLoudness,
        replaygainreferenceloudness
    );
    add_tests!(&TagKey::ReplayGainTrackGain, replaygaintrackgain);
    add_tests!(&TagKey::ReplayGainTrackPeak, replaygaintrackpeak);
    add_tests!(&TagKey::ReplayGainTrackRange, replaygaintrackrange);
    add_tests!(&TagKey::Script, script);
    add_tests!(&TagKey::ShowName, showname);
    add_tests!(&TagKey::ShowNameSortOrder, shownamesortorder);
    add_tests!(&TagKey::Subtitle, subtitle);
    add_tests!(&TagKey::TrackTitle, tracktitle);
    add_tests!(&TagKey::TrackTitleSortOrder, tracktitlesortorder);
    add_tests!(&TagKey::WorkTitle, worktitle);

    #[test]
    fn test_get_set_tracknumber() {
        let mut tag = Mp4Tag::new();
        assert!(tag.get(&TagKey::TrackNumber).is_none());
        tag.set(&TagKey::TrackNumber, Cow::from("7"));
        assert_eq!(tag.get(&TagKey::TrackNumber).as_deref(), Some("7"));
    }

    #[test]
    fn test_get_set_totaltracks() {
        let mut tag = Mp4Tag::new();
        assert!(tag.get(&TagKey::TotalTracks).is_none());
        tag.set(&TagKey::TotalTracks, Cow::from("12"));
        assert_eq!(tag.get(&TagKey::TotalTracks).as_deref(), Some("12"));
    }

    #[test]
    fn test_get_set_discnumber() {
        let mut tag = Mp4Tag::new();
        assert!(tag.get(&TagKey::DiscNumber).is_none());
        tag.set(&TagKey::DiscNumber, Cow::from("1"));
        assert_eq!(tag.get(&TagKey::DiscNumber).as_deref(), Some("1"));
    }

    #[test]
    fn test_get_set_totaldiscs() {
        let mut tag = Mp4Tag::new();
        assert!(tag.get(&TagKey::TotalDiscs).is_none());
        tag.set(&TagKey::TotalDiscs, Cow::from("2"));
        assert_eq!(tag.get(&TagKey::TotalDiscs).as_deref(), Some("2"));
    }

    #[test]
    fn test_get_set_bpm() {
        let mut tag = Mp4Tag::new();
        assert!(tag.get(&TagKey::Bpm).is_none());
        tag.set(&TagKey::Bpm, Cow::from("128"));
        assert_eq!(tag.get(&TagKey::Bpm).as_deref(), Some("128"));
    }

    #[test]
    fn test_get_set_compilation() {
        let mut tag = Mp4Tag::new();
        assert_eq!(tag.get(&TagKey::Compilation).as_deref(), Some("0"));
        tag.set(&TagKey::Compilation, Cow::from("1"));
        assert_eq!(tag.get(&TagKey::Compilation).as_deref(), Some("1"));
    }

    const PICARD_M4A: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/media/picard-2.12.3/track.m4a"
    );
    const UNTAGGED_M4A: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/media/untagged/track.m4a"
    );

    #[test]
    fn test_read_picard_file() {
        let tag = Mp4Tag::read_from_path(PICARD_M4A).unwrap();
        assert_eq!(tag.tag_type(), TagType::MP4);
        assert_eq!(
            tag.get(&TagKey::TrackTitle).as_deref(),
            Some("But Not for Me")
        );
        assert_eq!(
            tag.get(&TagKey::Artist).as_deref(),
            Some("The Ahmad Jamal Trio")
        );
        assert_eq!(tag.get(&TagKey::Genre).as_deref(), Some("Jazz"));
        assert_eq!(tag.get(&TagKey::TrackNumber).as_deref(), Some("1"));
        assert_eq!(tag.get(&TagKey::TotalTracks).as_deref(), Some("8"));
        assert_eq!(tag.get(&TagKey::DiscNumber).as_deref(), Some("1"));
        assert_eq!(tag.get(&TagKey::TotalDiscs).as_deref(), Some("1"));
        assert_eq!(tag.get(&TagKey::RecordLabel).as_deref(), Some("Argo"));
        assert_eq!(
            tag.get(&TagKey::MusicBrainzReleaseId).as_deref(),
            Some("0008f765-032b-46cd-ab69-2220edab1837")
        );
        assert_eq!(
            tag.get(&TagKey::ComposerSortOrder).as_deref(),
            Some("Gershwin, George")
        );
    }

    #[test]
    fn test_write_read_roundtrip() {
        let file = tempfile::Builder::new().suffix(".m4a").tempfile().unwrap();
        let _ = std::fs::copy(UNTAGGED_M4A, file.path()).unwrap();

        let mut tag = Mp4Tag::read_from_path(file.path()).unwrap();
        tag.set(&TagKey::TrackTitle, Cow::from("Roundtrip Title"));
        tag.set(&TagKey::MusicBrainzReleaseId, Cow::from("some-release-id"));
        tag.set_multiple(
            &TagKey::Artist,
            &[Cow::from("Artist A"), Cow::from("Artist B")],
        );
        tag.set(&TagKey::TrackNumber, Cow::from("3"));
        tag.set(&TagKey::TotalTracks, Cow::from("12"));
        tag.set(&TagKey::DiscNumber, Cow::from("2"));
        tag.set(&TagKey::TotalDiscs, Cow::from("4"));
        tag.set(&TagKey::Bpm, Cow::from("128"));
        tag.set(&TagKey::Compilation, Cow::from("1"));
        tag.write(file.path()).unwrap();

        let mut tag = Mp4Tag::read_from_path(file.path()).unwrap();
        assert_eq!(
            tag.get(&TagKey::TrackTitle).as_deref(),
            Some("Roundtrip Title")
        );
        assert_eq!(
            tag.get(&TagKey::MusicBrainzReleaseId).as_deref(),
            Some("some-release-id")
        );
        assert_eq!(tag.get(&TagKey::Artist).as_deref(), Some("Artist A"));
        assert_eq!(tag.get(&TagKey::TrackNumber).as_deref(), Some("3"));
        assert_eq!(tag.get(&TagKey::TotalTracks).as_deref(), Some("12"));
        assert_eq!(tag.get(&TagKey::DiscNumber).as_deref(), Some("2"));
        assert_eq!(tag.get(&TagKey::TotalDiscs).as_deref(), Some("4"));
        assert_eq!(tag.get(&TagKey::Bpm).as_deref(), Some("128"));
        assert_eq!(tag.get(&TagKey::Compilation).as_deref(), Some("1"));

        tag.clear(&TagKey::TrackTitle);
        tag.clear(&TagKey::MusicBrainzReleaseId);
        tag.clear(&TagKey::TotalTracks);
        tag.clear(&TagKey::Compilation);
        tag.write(file.path()).unwrap();

        let tag = Mp4Tag::read_from_path(file.path()).unwrap();
        assert!(tag.get(&TagKey::TrackTitle).is_none());
        assert!(tag.get(&TagKey::MusicBrainzReleaseId).is_none());
        assert!(tag.get(&TagKey::TotalTracks).is_none());
        assert_eq!(tag.get(&TagKey::TrackNumber).as_deref(), Some("3"));
        assert_eq!(tag.get(&TagKey::Compilation).as_deref(), Some("0"));
    }
}
