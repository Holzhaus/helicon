// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Generic release implementations.
use itertools::Itertools;
use musicbrainz_rs_nova::entity::artist::Artist as MusicBrainzArtist;
use musicbrainz_rs_nova::entity::relations::Relation as MusicBrainzRelation;
use musicbrainz_rs_nova::entity::relations::RelationContent as MusicBrainzRelationContent;
use musicbrainz_rs_nova::entity::release::Track as MusicBrainzReleaseTrack;
use musicbrainz_rs_nova::entity::work::Work as MusicBrainzWork;
use std::borrow::Cow;
use std::iter::Iterator;
use std::path::Path;

/// An person that was involved in this track's making.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvolvedPerson<'a> {
    /// The involvement (e.g., the instrument played or the role of the person)
    pub involvement: Cow<'a, str>,
    /// The name of the involved persoon.
    pub involvee: Cow<'a, str>,
}

/// Represent a generic release, independent of the underlying source.
pub trait TrackLike {
    /// AcoustID associated with the track.
    fn acoustid(&self) -> Option<Cow<'_, str>>;

    /// AcoustID Fingerprint for the track.
    fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>>;

    /// Artist who arranged the tune for performance.
    fn arranger(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Track Artist Name(s).
    fn track_artist(&self) -> Option<Cow<'_, str>>;

    /// Track Artist Sort Name.
    fn track_artist_sort_order(&self) -> Option<Cow<'_, str>>;

    /// Beats per minute of the track. Only available to the file naming script.
    fn bpm(&self) -> Option<Cow<'_, str>>;

    /// Comment.
    fn comment(&self) -> Option<Cow<'_, str>>;

    /// Composer Name(s).
    fn composer(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Composer Sort Name.
    fn composer_sort_order(&self) -> Option<Cow<'_, str>>;

    /// Conductor Name(s).
    fn conductor(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Contain copyright message for the copyright holder of the original sound, begin with a year and a space character.
    fn copyright(&self) -> Option<Cow<'_, str>>;

    /// The director of a video track as provided by the Video Director relationship in MusicBrainz.
    fn director(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// DJ-Mix Artist Name(s).
    ///
    /// This only applies to DJ-Mixes.
    fn dj_mixer(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Encoded by (person or organization).
    fn encoded_by(&self) -> Option<Cow<'_, str>>;

    /// Encoder Settings used.
    fn encoder_settings(&self) -> Option<Cow<'_, str>>;

    /// Recording Engineer Name(s).
    fn engineer(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Genre Name(s) of the track.
    fn genre(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Initial key of the track.
    fn initial_key(&self) -> Option<Cow<'_, str>>;

    /// International Standard Recording Code
    ///
    /// An international standard code for uniquely identifying sound recordings and music video
    /// recordings.
    fn isrc(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Work lyric language as per ISO 639-3.
    fn language(&self) -> Option<Cow<'_, str>>;

    /// License of the recording or release.
    fn license(&self) -> Option<Cow<'_, str>>;

    /// Lyricist Name(s).
    fn lyricist(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Lyrics.
    fn lyrics(&self) -> Option<Cow<'_, str>>;

    /// Mixing Engineer Name(s).
    fn mixer(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Mood.
    fn mood(&self) -> Option<Cow<'_, str>>;

    /// Movement.
    fn movement(&self) -> Option<Cow<'_, str>>;

    /// Movement Count.
    fn movement_count(&self) -> Option<Cow<'_, str>>;

    /// Movement Number.
    fn movement_number(&self) -> Option<Cow<'_, str>>;

    /// Track Artist’s MusicBrainz Identifier.
    fn musicbrainz_artist_id(&self) -> Option<Cow<'_, str>>;

    /// Original Track Artist’s MusicBrainz Identifier.
    fn musicbrainz_original_artist_id(&self) -> Option<Cow<'_, str>>;

    /// Original Release’s MusicBrainz Identifier.
    fn musicbrainz_original_release_id(&self) -> Option<Cow<'_, str>>;

    /// Recording’s MusicBrainz Identifier.
    fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>>;

    /// Release Track MusicBrainz Identifier.
    fn musicbrainz_track_id(&self) -> Option<Cow<'_, str>>;

    /// MusicBrainz TRM ID
    ///
    /// TRM (TRM Recognizes Music) was MusicBrainz' first audio fingerprinting system. Support for PUID was
    /// removed by MusicBrainz in 2008.
    fn musicbrainz_trm_id(&self) -> Option<Cow<'_, str>>;

    /// MusicBrainz Identifier for the work.
    fn musicbrainz_work_id(&self) -> Option<Cow<'_, str>>;

    /// MusicIP Fingerprint.
    ///
    /// MusicIP was MusicBrainz' second audio fingerprinting system. Support for PUID was
    /// removed by MusicBrainz in 2013.
    fn musicip_fingerprint(&self) -> Option<Cow<'_, str>>;

    /// MusicIP PUID.
    ///
    /// MusicIP was MusicBrainz' second audio fingerprinting system. Support for PUID was
    /// removed by MusicBrainz in 2013.
    fn musicip_puid(&self) -> Option<Cow<'_, str>>;

    /// Release Title of the earliest release in the Release Group intended for the title of the original recording.
    fn original_album(&self) -> Option<Cow<'_, str>>;

    /// Track Artist of the earliest release in the Release Group intended for the performer(s) of the original recording.
    fn original_artist(&self) -> Option<Cow<'_, str>>;

    /// Preferred File Name.
    ///
    /// The filename is case sensitive and includes its suffix.
    fn original_filename(&self) -> Option<Cow<'_, str>>;

    /// The original release date in the format YYYY-MM-DD.
    fn original_release_date(&self) -> Option<Cow<'_, str>>;

    /// The year of the original release date in the format YYYY.
    fn original_release_year(&self) -> Option<Cow<'_, str>>;

    /// Performer.
    fn performers(&self) -> Option<Vec<InvolvedPerson<'_>>>;

    /// Producer Name(s).
    fn producer(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Rating of the track.
    fn rating(&self) -> Option<Cow<'_, str>>;

    /// Remixer Name(s).
    fn remixer(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// ReplayGain Album Gain.
    fn replay_gain_album_gain(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Album Peak.
    fn replay_gain_album_peak(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Album Range.
    fn replay_gain_album_range(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Reference Loudness.
    fn replay_gain_reference_loudness(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Track Gain.
    fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Track Peak.
    fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Track Range.
    fn replay_gain_track_range(&self) -> Option<Cow<'_, str>>;

    /// Track number on the disc.
    fn track_number(&self) -> Option<Cow<'_, str>>;

    /// Track Title.
    fn track_title(&self) -> Option<Cow<'_, str>>;

    /// Track Title’s Sort Name.
    fn track_title_sort_order(&self) -> Option<Cow<'_, str>>;

    /// Used for official artist website.
    fn artist_website(&self) -> Option<Cow<'_, str>>;

    /// Title of the work.
    fn work_title(&self) -> Option<Cow<'_, str>>;

    /// Writer Name(s).
    ///
    /// This is used when uncertain whether the artist is the composer or the lyricist.
    fn writer(&self) -> impl Iterator<Item = Cow<'_, str>>;

    /// Track length.
    fn track_length(&self) -> Option<chrono::TimeDelta>;

    /// Analyzed metadata for this track.
    fn analyzed_metadata(&self) -> impl AnalyzedTrackMetadata {
        NoAnalyzedTrackMetadata
    }

    /// Path to the track in the filesystem.
    fn track_path(&self) -> Option<&Path> {
        None
    }

    /// File extension for this track.
    fn track_file_extension(&self) -> Option<Cow<'_, str>> {
        None
    }
}

/// Analyzed track metadata.
pub trait AnalyzedTrackMetadata {
    /// AcoustID Fingerprint for the track.
    fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Track Gain.
    fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Track Peak.
    fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>>;

    /// ReplayGain Track Range.
    fn replay_gain_track_range(&self) -> Option<Cow<'_, str>>;
}

/// Adds helper methods to the `MusicBrainzReleaseTrack` struct.
trait MusicBrainzReleaseTrackHelper {
    /// Get release relations by types.
    fn find_release_relations_by_type(
        &self,
        relation_types: &[&str],
    ) -> impl Iterator<Item = &MusicBrainzRelation>;

    /// Get the works for this track.
    fn find_works(&self) -> impl Iterator<Item = &Box<MusicBrainzWork>>;

    /// Get work relations by types.
    fn find_work_relations_by_type(
        &self,
        relation_types: &[&str],
    ) -> impl Iterator<Item = &MusicBrainzRelation>;

    /// Get release relation artists by relation types.
    fn find_release_relation_artists(
        &self,
        relation_types: &[&str],
    ) -> impl Iterator<Item = Cow<'_, str>>;
}

impl MusicBrainzReleaseTrackHelper for MusicBrainzReleaseTrack {
    fn find_release_relations_by_type(
        &self,
        relation_types: &[&str],
    ) -> impl Iterator<Item = &MusicBrainzRelation> {
        self.recording
            .iter()
            .flat_map(|recording| recording.relations.iter())
            .flat_map(|relations| relations.iter())
            .filter(|relation| relation_types.contains(&relation.relation_type.as_str()))
    }

    fn find_works(&self) -> impl Iterator<Item = &Box<MusicBrainzWork>> {
        self.find_release_relations_by_type(&["performance"])
            .filter(|relation| relation.direction.as_str() == "forward")
            .filter_map(|relation| {
                if let MusicBrainzRelationContent::Work(work) = &relation.content {
                    Some(work)
                } else {
                    None
                }
            })
    }

    fn find_work_relations_by_type(
        &self,
        relation_types: &[&str],
    ) -> impl Iterator<Item = &MusicBrainzRelation> {
        self.find_works()
            .flat_map(|work| work.relations.iter())
            .flat_map(|relations| relations.iter())
            .filter(|relation| relation_types.contains(&relation.relation_type.as_str()))
    }

    fn find_release_relation_artists(
        &self,
        relation_types: &[&str],
    ) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relations_by_type(relation_types)
            .filter_map(relation_artist)
            .map(|artist| &artist.name)
            .map(Cow::from)
    }
}

/// Helper method to get the artist from a relation.
fn relation_artist(relation: &MusicBrainzRelation) -> Option<&MusicBrainzArtist> {
    if let MusicBrainzRelationContent::Artist(artist) = &relation.content {
        Some(artist)
    } else {
        None
    }
}

impl TrackLike for MusicBrainzReleaseTrack {
    fn acoustid(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn arranger(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relations_by_type(&[
            "arranger",
            "instrument arranger",
            "orchestrator",
            "vocal arranger",
        ])
        .filter_map(relation_artist)
        .map(|artist| &artist.name)
        .map(Cow::from)
    }

    fn track_artist(&self) -> Option<Cow<'_, str>> {
        Cow::from(
            self.artist_credit
                .as_deref()
                .or(self
                    .recording
                    .as_ref()
                    .and_then(|recording| recording.artist_credit.as_deref()))
                .iter()
                .flat_map(|artists| artists.iter())
                .fold(String::new(), |acc, artist| {
                    acc + &artist.name
                        + if let Some(joinphrase) = &artist.joinphrase {
                            joinphrase
                        } else {
                            ""
                        }
                }),
        )
        .into()
    }

    fn track_artist_sort_order(&self) -> Option<Cow<'_, str>> {
        Cow::from(
            self.artist_credit
                .as_deref()
                .or(self
                    .recording
                    .as_ref()
                    .and_then(|recording| recording.artist_credit.as_deref()))
                .iter()
                .flat_map(|artists| artists.iter())
                .map(|artist| &artist.artist)
                .fold(String::new(), |acc, artist| {
                    if acc.is_empty() {
                        acc + &artist.sort_name
                    } else {
                        acc + ";" + &artist.sort_name
                    }
                }),
        )
        .into()
    }

    fn bpm(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn comment(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn composer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relations_by_type(&["composition", "composer"])
            .chain(self.find_work_relations_by_type(&["composition", "composer"]))
            .filter_map(relation_artist)
            .map(|artist| &artist.name)
            .map(Cow::from)
    }

    fn composer_sort_order(&self) -> Option<Cow<'_, str>> {
        Some(
            self.find_release_relations_by_type(&["composition", "composer"])
                .chain(self.find_work_relations_by_type(&["composition", "composer"]))
                .filter_map(relation_artist)
                .map(|artist| &artist.sort_name)
                .join(";"),
        )
        .filter(|s| !s.is_empty())
        .map(Cow::from)
    }

    fn conductor(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relation_artists(&["conductor"])
    }

    fn copyright(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn director(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relation_artists(&[
            "audio director",
            "video director",
            "creative direction",
            "art direction",
        ])
    }

    fn dj_mixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relation_artists(&["mix-DJ"])
    }

    fn encoded_by(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn encoder_settings(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn engineer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relation_artists(&[
            "engineer",
            "audio",
            "mastering",
            "sound",
            "mix",
            "recording",
            "field recordist",
            "programming",
            "editor",
            "balance",
        ])
    }

    fn genre(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.recording
            .iter()
            .flat_map(|recording| recording.genres.iter())
            .flat_map(|genres| genres.iter())
            .map(|genre| Cow::from(&genre.name))
    }

    fn initial_key(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn isrc(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.recording
            .iter()
            .flat_map(|recording| recording.isrcs.iter())
            .flat_map(|isrcs| isrcs.iter())
            .map(Cow::from)
    }

    fn language(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn license(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn lyricist(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relations_by_type(&["lyricist"])
            .chain(self.find_work_relations_by_type(&["lyricist"]))
            .filter_map(relation_artist)
            .map(|artist| &artist.name)
            .map(Cow::from)
    }

    fn lyrics(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn mixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relation_artists(&["mix"])
    }

    fn mood(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn movement(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn movement_count(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn movement_number(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn musicbrainz_artist_id(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn musicbrainz_original_artist_id(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn musicbrainz_original_release_id(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>> {
        self.recording
            .as_ref()
            .map(|recording| Cow::from(&recording.id))
    }

    fn musicbrainz_track_id(&self) -> Option<Cow<'_, str>> {
        Cow::from(self.id.as_str()).into()
    }

    fn musicbrainz_trm_id(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn musicbrainz_work_id(&self) -> Option<Cow<'_, str>> {
        self.find_works().map(|work| &work.id).map(Cow::from).next()
    }

    fn musicip_fingerprint(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn musicip_puid(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn original_album(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn original_artist(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn original_filename(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn original_release_date(&self) -> Option<Cow<'_, str>> {
        self.recording
            .as_ref()
            .and_then(|recording| recording.first_release_date)
            .map(|date| Cow::from(date.format("%Y-%m-%d").to_string()))
    }

    fn original_release_year(&self) -> Option<Cow<'_, str>> {
        self.recording
            .as_ref()
            .and_then(|recording| recording.first_release_date)
            .map(|date| Cow::from(date.format("%Y").to_string()))
    }

    fn performers(&self) -> Option<Vec<InvolvedPerson<'_>>> {
        self.find_release_relations_by_type(&["performer", "instrument", "vocal"])
            .filter_map(|relation| {
                if let MusicBrainzRelationContent::Artist(artist) = &relation.content {
                    Some((&artist.name, &relation.attributes))
                } else {
                    None
                }
            })
            .map(|(artist, attributes)| {
                let attrs = attributes.iter().flat_map(|vec| vec.iter()).join(", ");
                InvolvedPerson {
                    involvement: Cow::from(attrs),
                    involvee: Cow::from(artist),
                }
            })
            .collect::<Vec<_>>()
            .into()
    }

    fn producer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relation_artists(&["producer"])
    }

    fn rating(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn remixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relation_artists(&["remixer"])
    }

    fn replay_gain_album_gain(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn replay_gain_album_peak(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn replay_gain_album_range(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn replay_gain_reference_loudness(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn replay_gain_track_range(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn track_number(&self) -> Option<Cow<'_, str>> {
        Cow::from(&self.number).into()
    }

    fn track_title(&self) -> Option<Cow<'_, str>> {
        Cow::from(self.title.as_str()).into()
    }

    fn track_title_sort_order(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn artist_website(&self) -> Option<Cow<'_, str>> {
        // TODO: Implement this.
        None
    }

    fn work_title(&self) -> Option<Cow<'_, str>> {
        self.find_works()
            .map(|work| &work.title)
            .map(Cow::from)
            .next()
    }

    fn writer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.find_release_relations_by_type(&["writer"])
            .chain(self.find_work_relations_by_type(&["writer"]))
            .filter_map(relation_artist)
            .map(|artist| &artist.name)
            .map(Cow::from)
    }

    fn track_length(&self) -> Option<chrono::TimeDelta> {
        self.length
            .map(|length| chrono::TimeDelta::milliseconds(length.into()))
    }
}

/// Default struct when is no metadata is analyzed for this track.
struct NoAnalyzedTrackMetadata;

impl AnalyzedTrackMetadata for NoAnalyzedTrackMetadata {
    fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn replay_gain_track_range(&self) -> Option<Cow<'_, str>> {
        None
    }
}
