// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Tags and tag-related functions.

#[cfg(feature = "flac")]
mod flac;
#[cfg(feature = "id3")]
mod id3;

use std::path::Path;

/// A tag key describes the kind of information in a generic, format-independent way.
#[allow(dead_code)]
pub enum TagKey {
    /// AcoustID associated with the track.
    AcoustId,
    /// AcoustID Fingerprint for the track.
    AcoustIdFingerprint,
    /// Title of the release.
    Album,
    /// Artist(s) primarily credited on the release.
    AlbumArtist,
    /// Release Artist’s Sort Name (e.g.: “Beatles, The”).
    AlbumArtistSortOrder,
    /// Release Title’s Sort Name.
    AlbumSortOrder,
    /// Artist who arranged the tune for performance.
    Arranger,
    /// Track Artist Name(s).
    Artist,
    /// Track Artist Sort Name.
    ArtistSortOrder,
    /// Track Artist Name(s).
    Artists,
    /// Amazon Standard Identification Number - the number identifying the item on Amazon.
    Asin,
    /// Release Barcode - the barcode assigned to the release.
    Barcode,
    /// Beats per minute of the track. Only available to the file naming script.
    Bpm,
    /// The number(s) assigned to the release by the label(s), which can often be found on the
    /// spine or near the barcode. There may be more than one, especially when multiple labels are
    /// involved.
    CatalogNumber,
    /// Comment.
    Comment,
    /// 1 for Various Artist albums, otherwise 0 (compatible with iTunes).
    Compilation,
    /// Composer Name(s).
    Composer,
    /// Composer Sort Name.
    ComposerSortOrder,
    /// Conductor Name(s).
    Conductor,
    /// Contain copyright message for the copyright holder of the original sound, begin with a year and a space character.
    Copyright,
    /// The director of a video track as provided by the Video Director relationship in MusicBrainz.
    Director,
    /// Number of the disc in this release that contains this track.
    DiscNumber,
    /// The Media Title given to a specific disc.
    DiscSubtitle,
    /// Encoded by (person or organization).
    EncodedBy,
    /// Encoder Settings used.
    EncoderSettings,
    /// Recording Engineer Name(s).
    Engineer,
    /// Indicated if the playback is gapless.
    GaplessPlayback,
    /// Genre Name(s) of the track.
    Genre,
    /// Content Group.
    Grouping,
    /// Initial key of the track.
    InitialKey,
    /// International Standard Recording Code
    ///
    /// An international standard code for uniquely identifying sound recordings and music video
    /// recordings.
    Isrc,
    /// Work lyric language as per ISO 639-3.
    Language,
    /// License of the recording or release.
    License,
    /// Lyricist Name(s).
    Lyricist,
    /// Lyrics.
    Lyrics,
    /// Release Format (e.g.: CD).
    Media,
    /// DJ-Mix Artist Name(s).
    ///
    /// This only applies to DJ-Mixes.
    DjMixer,
    /// Mixing Engineer Name(s).
    Mixer,
    /// Mood.
    Mood,
    /// Movement.
    Movement,
    /// Movement Count.
    MovementCount,
    /// Movement Number.
    MovementNumber,
    /// Track Artist’s MusicBrainz Identifier.
    MusicBrainzArtistId,
    /// Disc ID is the code number which MusicBrainz uses to link a physical CD to a release
    /// listing. This is based on the table of contents (TOC) information read from the disc. This
    /// tag contains the Disc ID if the album information was retrieved using “Tools ‣ Lookup CD”.
    MusicBrainzDiscId,
    /// Original Track Artist’s MusicBrainz Identifier.
    MusicBrainzOriginalArtistId,
    /// Original Release’s MusicBrainz Identifier.
    MusicBrainzOriginalReleaseId,
    /// Recording’s MusicBrainz Identifier.
    MusicBrainzRecordingId,
    /// Release Artist’s MusicBrainz Identifier.
    MusicBrainzReleaseArtistId,
    /// Release Group’s MusicBrainz Identifier.
    MusicBrainzReleaseGroupId,
    /// Release MusicBrainz Identifier.
    MusicBrainzReleaseId,
    /// Release Track MusicBrainz Identifier.
    MusicBrainzTrackId,
    /// MusicBrainz TRM ID
    ///
    /// TRM (TRM Recognizes Music) was MusicBrainz' first audio fingerprinting system. Support for PUID was
    /// removed by MusicBrainz in 2008.
    MusicBrainzTrmId,
    /// MusicBrainz Identifier for the work.
    MusicBrainzWorkId,
    /// MusicIP Fingerprint.
    ///
    /// MusicIP was MusicBrainz' second audio fingerprinting system. Support for PUID was
    /// removed by MusicBrainz in 2013.
    MusicIpFingerprint,
    /// MusicIP PUID.
    ///
    /// MusicIP was MusicBrainz' second audio fingerprinting system. Support for PUID was
    /// removed by MusicBrainz in 2013.
    MusicIpPuid,
    /// Release Title of the earliest release in the Release Group intended for the title of the original recording.
    OriginalAlbum,
    /// Track Artist of the earliest release in the Release Group intended for the performer(s) of the original recording.
    OriginalArtist,
    /// Preferred File Name.
    ///
    /// The filename is case sensitive and includes its suffix.
    OriginalFilename,
    /// The original release date in the format YYYY-MM-DD.
    OriginalReleaseDate,
    /// The year of the original release date in the format YYYY.
    OriginalReleaseYear,
    /// Performer.
    Performer,
    /// Podcast.
    Podcast,
    /// Podcast URL.
    PodcastUrl,
    /// Producer Nae(s).
    Producer,
    /// Rating of the track.
    Rating,
    /// Release Record Label Name(s).
    RecordLabel,
    /// Country in which the release was issued.
    ReleaseCountry,
    /// Release Date (YYYY-MM-DD) - the date that the release was issued.
    ReleaseDate,
    /// Release Year (YYYY) - the year that the release was issued.
    ReleaseYear,
    /// Release Status indicating the “official” status of the release.
    ReleaseStatus,
    /// Release Group Type.
    ReleaseType,
    /// Remixer Name(s).
    Remixer,
    /// ReplayGain Album Gain.
    ReplayGainAlbumGain,
    /// ReplayGain Album Peak.
    ReplayGainAlbumPeak,
    /// ReplayGain Album Range.
    ReplayGainAlbumRange,
    /// ReplayGain Reference Loudness.
    ReplayGainReferenceLoudness,
    /// ReplayGain Track Gain.
    ReplayGainTrackGain,
    /// ReplayGain Track Peak.
    ReplayGainTrackPeak,
    /// ReplayGain Track Range.
    ReplayGainTrackRange,
    /// The script used to write the release’s track list.
    ///
    /// The values should be taken from the ISO 15924 standard.
    Script,
    /// Show Name.
    ShowName,
    /// Show Name Sort Order.
    ShowNameSortOrder,
    /// Show Work & Movement.
    ShowMovement,
    /// Used for information directly related to the contents title.
    Subtitle,
    /// Total number of discs in this release.
    TotalDiscs,
    /// Total tracks on this disc.
    TotalTracks,
    /// Track number on the disc.
    TrackNumber,
    /// Track Title.
    TrackTitle,
    /// Track Title’s Sort Name.
    TrackTitleSortOrder,
    /// Used for official artist website.
    ArtistWebsite,
    /// Title of the work.
    WorkTitle,
    /// Writer Name(s).
    ///
    /// This is used when uncertain whether the artist is the composer or the lyricist.
    Writer,
}

/// The tag type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TagType {
    /// ID3v2.2 tag
    ID3v22,
    /// ID3v2.3 tag
    ID3v23,
    /// ID3v2.3 tag
    ID3v24,
    /// Vorbis tag from a FLAC file
    Flac,
}

/// A tag tag can be used for reading.
pub trait Tag {
    /// Get the tag type.
    fn tag_type(&self) -> TagType;
    /// Get the string value for the tag key.
    fn get(&self, key: &TagKey) -> Option<&str>;
}

/// A tagged file that contains zero or more tags.
pub struct TaggedFile {
    /// Tags that are present in the file.
    content: Vec<Box<dyn Tag>>,
}

impl TaggedFile {
    /// Creates a [`TaggedFile`] from the path.
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Self> {
        path.as_ref()
            .extension()
            .map(std::ffi::OsStr::to_ascii_lowercase)
            .ok_or(crate::Error::UnknownFileType)
            .and_then(|extension| {
                extension
                    .to_str()
                    .ok_or(crate::Error::UnknownFileType)
                    .map(|ext| match ext {
                        #[cfg(feature = "id3")]
                        "mp3" => self::id3::ID3v2Tag::read_from_path(&path)
                            .map(Box::new)
                            .map(|tag| Box::<dyn Tag>::from(tag))
                            .map(|tag| vec![tag]),
                        #[cfg(feature = "flac")]
                        "flac" => self::flac::FlacTag::read_from_path(&path)
                            .map(Box::new)
                            .map(|tag| Box::<dyn Tag>::from(tag))
                            .map(|tag| vec![tag]),
                        ext => {
                            log::debug!("Unknown file extension {:?}", ext);
                            Err(crate::Error::UnknownFileType)
                        }
                    })?
            })
            .map(|content| Self { content })
    }

    /// Returns zero or more [`Tag`] objects.
    pub fn tags(&self) -> &[Box<dyn Tag>] {
        &self.content
    }
}
