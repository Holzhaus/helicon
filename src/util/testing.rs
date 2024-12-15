// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

#![cfg(any(test, feature = "dev"))]
//! Testing utils.

use crate::media::MediaLike;
use crate::release::ReleaseLike;
use crate::track::InvolvedPerson;
use crate::track::TrackLike;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A fake release.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FakeRelease {
    release_title: Option<String>,
    release_artist: Option<String>,
    release_artist_sort_order: Option<String>,
    release_sort_order: Option<String>,
    asin: Option<String>,
    barcode: Option<String>,
    catalog_number: Option<String>,
    compilation: Option<String>,
    grouping: Option<String>,
    musicbrainz_release_artist_id: Option<String>,
    musicbrainz_release_group_id: Option<String>,
    musicbrainz_release_id: Option<String>,
    record_label: Option<String>,
    release_country: Option<String>,
    release_date: Option<String>,
    release_year: Option<String>,
    release_status: Option<String>,
    release_type: Option<String>,
    script: Option<String>,
    total_discs: Option<String>,
    replay_gain_album_gain_analyzed: Option<String>,
    replay_gain_album_peak_analyzed: Option<String>,
    media: Vec<FakeMedia>,
    is_compilation: bool,
}

impl<T> From<&T> for FakeRelease
where
    T: ReleaseLike,
{
    fn from(release: &T) -> Self {
        Self {
            release_title: release.release_title().as_deref().map(ToString::to_string),
            release_artist: release.release_artist().as_deref().map(ToString::to_string),
            release_artist_sort_order: release
                .release_artist_sort_order()
                .as_deref()
                .map(ToString::to_string),
            release_sort_order: release
                .release_sort_order()
                .as_deref()
                .map(ToString::to_string),
            asin: release.asin().as_deref().map(ToString::to_string),
            barcode: release.barcode().as_deref().map(ToString::to_string),
            catalog_number: release.catalog_number().as_deref().map(ToString::to_string),
            compilation: release.compilation().as_deref().map(ToString::to_string),
            grouping: release.grouping().as_deref().map(ToString::to_string),
            musicbrainz_release_artist_id: release
                .musicbrainz_release_artist_id()
                .as_deref()
                .map(ToString::to_string),
            musicbrainz_release_group_id: release
                .musicbrainz_release_group_id()
                .as_deref()
                .map(ToString::to_string),
            musicbrainz_release_id: release
                .musicbrainz_release_id()
                .as_deref()
                .map(ToString::to_string),
            record_label: release.record_label().as_deref().map(ToString::to_string),
            release_country: release
                .release_country()
                .as_deref()
                .map(ToString::to_string),
            release_date: release.release_date().as_deref().map(ToString::to_string),
            release_year: release.release_year().as_deref().map(ToString::to_string),
            release_status: release.release_status().as_deref().map(ToString::to_string),
            release_type: release.release_type().as_deref().map(ToString::to_string),
            script: release.script().as_deref().map(ToString::to_string),
            total_discs: release.total_discs().as_deref().map(ToString::to_string),
            replay_gain_album_gain_analyzed: release
                .replay_gain_album_gain_analyzed()
                .as_deref()
                .map(ToString::to_string),
            replay_gain_album_peak_analyzed: release
                .replay_gain_album_peak_analyzed()
                .as_deref()
                .map(ToString::to_string),
            media: release.media().map(FakeMedia::from).collect(),
            is_compilation: release.is_compilation(),
        }
    }
}

impl ReleaseLike for FakeRelease {
    fn release_title(&self) -> Option<Cow<'_, str>> {
        self.release_title.as_deref().map(Cow::from)
    }

    fn release_artist(&self) -> Option<Cow<'_, str>> {
        self.release_artist.as_deref().map(Cow::from)
    }

    fn release_artist_sort_order(&self) -> Option<Cow<'_, str>> {
        self.release_artist_sort_order.as_deref().map(Cow::from)
    }

    fn release_sort_order(&self) -> Option<Cow<'_, str>> {
        self.release_sort_order.as_deref().map(Cow::from)
    }

    fn asin(&self) -> Option<Cow<'_, str>> {
        self.asin.as_deref().map(Cow::from)
    }

    fn barcode(&self) -> Option<Cow<'_, str>> {
        self.barcode.as_deref().map(Cow::from)
    }

    fn catalog_number(&self) -> Option<Cow<'_, str>> {
        self.catalog_number.as_deref().map(Cow::from)
    }

    fn compilation(&self) -> Option<Cow<'_, str>> {
        self.compilation.as_deref().map(Cow::from)
    }

    fn grouping(&self) -> Option<Cow<'_, str>> {
        self.grouping.as_deref().map(Cow::from)
    }

    fn musicbrainz_release_artist_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_release_artist_id.as_deref().map(Cow::from)
    }

    fn musicbrainz_release_group_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_release_group_id.as_deref().map(Cow::from)
    }

    fn musicbrainz_release_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_release_id.as_deref().map(Cow::from)
    }

    fn record_label(&self) -> Option<Cow<'_, str>> {
        self.record_label.as_deref().map(Cow::from)
    }

    fn release_country(&self) -> Option<Cow<'_, str>> {
        self.release_country.as_deref().map(Cow::from)
    }

    fn release_date(&self) -> Option<Cow<'_, str>> {
        self.release_date.as_deref().map(Cow::from)
    }

    fn release_year(&self) -> Option<Cow<'_, str>> {
        self.release_year.as_deref().map(Cow::from)
    }

    fn release_status(&self) -> Option<Cow<'_, str>> {
        self.release_status.as_deref().map(Cow::from)
    }

    fn release_type(&self) -> Option<Cow<'_, str>> {
        self.release_type.as_deref().map(Cow::from)
    }

    fn script(&self) -> Option<Cow<'_, str>> {
        self.script.as_deref().map(Cow::from)
    }

    fn total_discs(&self) -> Option<Cow<'_, str>> {
        self.total_discs.as_deref().map(Cow::from)
    }

    fn media(&self) -> impl Iterator<Item = &(impl MediaLike + '_)> {
        self.media.iter()
    }

    fn replay_gain_album_gain_analyzed(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_album_gain_analyzed
            .as_deref()
            .map(Cow::from)
    }

    fn replay_gain_album_peak_analyzed(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_album_peak_analyzed
            .as_deref()
            .map(Cow::from)
    }

    fn is_compilation(&self) -> bool {
        self.is_compilation
    }
}

/// A fake media.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FakeMedia {
    disc_number: Option<u32>,
    media_title: Option<String>,
    media_format: Option<String>,
    musicbrainz_disc_id: Option<String>,
    media_tracks: Vec<FakeTrack>,
    gapless_playback: Option<bool>,
}

impl<T> From<&T> for FakeMedia
where
    T: MediaLike,
{
    fn from(media: &T) -> Self {
        Self {
            disc_number: media.disc_number(),
            media_title: media.media_title().as_deref().map(ToString::to_string),
            media_format: media.media_format().as_deref().map(ToString::to_string),
            musicbrainz_disc_id: media
                .musicbrainz_disc_id()
                .as_deref()
                .map(ToString::to_string),
            media_tracks: media.media_tracks().map(FakeTrack::from).collect(),
            gapless_playback: media.gapless_playback(),
        }
    }
}

impl MediaLike for FakeMedia {
    fn disc_number(&self) -> Option<u32> {
        self.disc_number
    }

    fn media_title(&self) -> Option<Cow<'_, str>> {
        self.media_title.as_deref().map(Cow::from)
    }

    fn media_format(&self) -> Option<Cow<'_, str>> {
        self.media_format.as_deref().map(Cow::from)
    }

    fn musicbrainz_disc_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_disc_id.as_deref().map(Cow::from)
    }

    fn media_track_count(&self) -> Option<usize> {
        Some(self.media_tracks.len())
    }

    fn media_tracks(&self) -> impl Iterator<Item = &(impl TrackLike + '_)> {
        self.media_tracks.iter()
    }

    fn gapless_playback(&self) -> Option<bool> {
        self.gapless_playback
    }
}

/// A fake track.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FakeTrack {
    acoustid: Option<String>,
    acoustid_fingerprint: Option<String>,
    arranger: Vec<String>,
    track_artist: Option<String>,
    track_artist_sort_order: Option<String>,
    bpm: Option<String>,
    comment: Option<String>,
    composer: Vec<String>,
    composer_sort_order: Option<String>,
    conductor: Vec<String>,
    copyright: Option<String>,
    director: Vec<String>,
    dj_mixer: Vec<String>,
    encoded_by: Option<String>,
    encoder_settings: Option<String>,
    engineer: Vec<String>,
    genre: Vec<String>,
    initial_key: Option<String>,
    isrc: Vec<String>,
    language: Option<String>,
    license: Option<String>,
    lyricist: Vec<String>,
    lyrics: Option<String>,
    mixer: Vec<String>,
    mood: Option<String>,
    movement: Option<String>,
    movement_count: Option<String>,
    movement_number: Option<String>,
    musicbrainz_artist_id: Option<String>,
    musicbrainz_original_artist_id: Option<String>,
    musicbrainz_original_release_id: Option<String>,
    musicbrainz_recording_id: Option<String>,
    musicbrainz_track_id: Option<String>,
    musicbrainz_trm_id: Option<String>,
    musicbrainz_work_id: Option<String>,
    musicip_fingerprint: Option<String>,
    musicip_puid: Option<String>,
    original_album: Option<String>,
    original_artist: Option<String>,
    original_filename: Option<String>,
    original_release_date: Option<String>,
    original_release_year: Option<String>,
    performers: Option<Vec<(String, String)>>,
    producer: Vec<String>,
    rating: Option<String>,
    remixer: Vec<String>,
    replay_gain_album_gain: Option<String>,
    replay_gain_album_peak: Option<String>,
    replay_gain_album_range: Option<String>,
    replay_gain_reference_loudness: Option<String>,
    replay_gain_track_gain: Option<String>,
    replay_gain_track_peak: Option<String>,
    replay_gain_track_range: Option<String>,
    track_number: Option<String>,
    track_title: Option<String>,
    track_title_sort_order: Option<String>,
    artist_website: Option<String>,
    work_title: Option<String>,
    writer: Vec<String>,
    track_length: Option<(i64, u32)>,
}

impl FakeTrack {
    #[cfg(test)]
    /// Convenience function to create a fake track with the given title.
    ///
    /// All other fields will be empty or unset.
    pub fn with_title(title: &(impl ToString + ?Sized)) -> Self {
        Self {
            track_title: Some(title.to_string()),
            ..Default::default()
        }
    }
}

impl<T> From<&T> for FakeTrack
where
    T: TrackLike,
{
    fn from(track: &T) -> Self {
        Self {
            acoustid: track.acoustid().as_deref().map(ToString::to_string),
            acoustid_fingerprint: track
                .acoustid_fingerprint()
                .as_deref()
                .map(ToString::to_string),
            arranger: track.arranger().map(|v| v.to_string()).collect(),
            track_artist: track.track_artist().as_deref().map(ToString::to_string),
            track_artist_sort_order: track
                .track_artist_sort_order()
                .as_deref()
                .map(ToString::to_string),
            bpm: track.bpm().as_deref().map(ToString::to_string),
            comment: track.comment().as_deref().map(ToString::to_string),
            composer: track.composer().map(|v| v.to_string()).collect(),
            composer_sort_order: track
                .composer_sort_order()
                .as_deref()
                .map(ToString::to_string),
            conductor: track.conductor().map(|v| v.to_string()).collect(),
            copyright: track.copyright().as_deref().map(ToString::to_string),
            director: track.director().map(|v| v.to_string()).collect(),
            dj_mixer: track.dj_mixer().map(|v| v.to_string()).collect(),
            encoded_by: track.encoded_by().as_deref().map(ToString::to_string),
            encoder_settings: track.encoder_settings().as_deref().map(ToString::to_string),
            engineer: track.engineer().map(|v| v.to_string()).collect(),
            genre: track.genre().map(|v| v.to_string()).collect(),
            initial_key: track.initial_key().as_deref().map(ToString::to_string),
            isrc: track.isrc().map(|v| v.to_string()).collect(),
            language: track.language().as_deref().map(ToString::to_string),
            license: track.license().as_deref().map(ToString::to_string),
            lyricist: track.lyricist().map(|v| v.to_string()).collect(),
            lyrics: track.lyrics().as_deref().map(ToString::to_string),
            mixer: track.mixer().map(|v| v.to_string()).collect(),
            mood: track.mood().as_deref().map(ToString::to_string),
            movement: track.movement().as_deref().map(ToString::to_string),
            movement_count: track.movement_count().as_deref().map(ToString::to_string),
            movement_number: track.movement_number().as_deref().map(ToString::to_string),
            musicbrainz_artist_id: track
                .musicbrainz_artist_id()
                .as_deref()
                .map(ToString::to_string),
            musicbrainz_original_artist_id: track
                .musicbrainz_original_artist_id()
                .as_deref()
                .map(ToString::to_string),
            musicbrainz_original_release_id: track
                .musicbrainz_original_release_id()
                .as_deref()
                .map(ToString::to_string),
            musicbrainz_recording_id: track
                .musicbrainz_recording_id()
                .as_deref()
                .map(ToString::to_string),
            musicbrainz_track_id: track
                .musicbrainz_track_id()
                .as_deref()
                .map(ToString::to_string),
            musicbrainz_trm_id: track
                .musicbrainz_trm_id()
                .as_deref()
                .map(ToString::to_string),
            musicbrainz_work_id: track
                .musicbrainz_work_id()
                .as_deref()
                .map(ToString::to_string),
            musicip_fingerprint: track
                .musicip_fingerprint()
                .as_deref()
                .map(ToString::to_string),
            musicip_puid: track.musicip_puid().as_deref().map(ToString::to_string),
            original_album: track.original_album().as_deref().map(ToString::to_string),
            original_artist: track.original_artist().as_deref().map(ToString::to_string),
            original_filename: track
                .original_filename()
                .as_deref()
                .map(ToString::to_string),
            original_release_date: track
                .original_release_date()
                .as_deref()
                .map(ToString::to_string),
            original_release_year: track
                .original_release_year()
                .as_deref()
                .map(ToString::to_string),
            performers: track.performers().map(|persons| {
                persons
                    .iter()
                    .map(|v| (v.involvement.to_string(), v.involvee.to_string()))
                    .collect()
            }),
            producer: track.producer().map(|v| v.to_string()).collect(),
            rating: track.rating().as_deref().map(ToString::to_string),
            remixer: track.remixer().map(|v| v.to_string()).collect(),
            replay_gain_album_gain: track
                .replay_gain_album_gain()
                .as_deref()
                .map(ToString::to_string),
            replay_gain_album_peak: track
                .replay_gain_album_peak()
                .as_deref()
                .map(ToString::to_string),
            replay_gain_album_range: track
                .replay_gain_album_range()
                .as_deref()
                .map(ToString::to_string),
            replay_gain_reference_loudness: track
                .replay_gain_reference_loudness()
                .as_deref()
                .map(ToString::to_string),
            replay_gain_track_gain: track
                .replay_gain_track_gain()
                .as_deref()
                .map(ToString::to_string),
            replay_gain_track_peak: track
                .replay_gain_track_peak()
                .as_deref()
                .map(ToString::to_string),
            replay_gain_track_range: track
                .replay_gain_track_range()
                .as_deref()
                .map(ToString::to_string),
            track_number: track.track_number().as_deref().map(ToString::to_string),
            track_title: track.track_title().as_deref().map(ToString::to_string),
            track_title_sort_order: track
                .track_title_sort_order()
                .as_deref()
                .map(ToString::to_string),
            artist_website: track.artist_website().as_deref().map(ToString::to_string),
            work_title: track.work_title().as_deref().map(ToString::to_string),
            writer: track.writer().map(|v| v.to_string()).collect(),
            track_length: track.track_length().and_then(|track_length| {
                u32::try_from(track_length.subsec_nanos())
                    .ok()
                    .map(|subsec_nanos| (track_length.num_seconds(), subsec_nanos))
            }),
        }
    }
}

impl TrackLike for FakeTrack {
    fn acoustid(&self) -> Option<Cow<'_, str>> {
        self.acoustid.as_deref().map(Cow::from)
    }

    fn acoustid_fingerprint(&self) -> Option<Cow<'_, str>> {
        self.acoustid_fingerprint.as_deref().map(Cow::from)
    }

    fn arranger(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.arranger.iter().map(Cow::from)
    }

    fn track_artist(&self) -> Option<Cow<'_, str>> {
        self.track_artist.as_deref().map(Cow::from)
    }

    fn track_artist_sort_order(&self) -> Option<Cow<'_, str>> {
        self.track_artist_sort_order.as_deref().map(Cow::from)
    }

    fn bpm(&self) -> Option<Cow<'_, str>> {
        self.bpm.as_deref().map(Cow::from)
    }

    fn comment(&self) -> Option<Cow<'_, str>> {
        self.comment.as_deref().map(Cow::from)
    }

    fn composer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.composer.iter().map(Cow::from)
    }

    fn composer_sort_order(&self) -> Option<Cow<'_, str>> {
        self.composer_sort_order.as_deref().map(Cow::from)
    }

    fn conductor(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.conductor.iter().map(Cow::from)
    }

    fn copyright(&self) -> Option<Cow<'_, str>> {
        self.copyright.as_deref().map(Cow::from)
    }

    fn director(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.director.iter().map(Cow::from)
    }

    fn dj_mixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.dj_mixer.iter().map(Cow::from)
    }

    fn encoded_by(&self) -> Option<Cow<'_, str>> {
        self.encoded_by.as_deref().map(Cow::from)
    }

    fn encoder_settings(&self) -> Option<Cow<'_, str>> {
        self.encoder_settings.as_deref().map(Cow::from)
    }

    fn engineer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.engineer.iter().map(Cow::from)
    }

    fn genre(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.genre.iter().map(Cow::from)
    }

    fn initial_key(&self) -> Option<Cow<'_, str>> {
        self.initial_key.as_deref().map(Cow::from)
    }

    fn isrc(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.isrc.iter().map(Cow::from)
    }

    fn language(&self) -> Option<Cow<'_, str>> {
        self.language.as_deref().map(Cow::from)
    }

    fn license(&self) -> Option<Cow<'_, str>> {
        self.license.as_deref().map(Cow::from)
    }

    fn lyricist(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.lyricist.iter().map(Cow::from)
    }

    fn lyrics(&self) -> Option<Cow<'_, str>> {
        self.lyrics.as_deref().map(Cow::from)
    }

    fn mixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.mixer.iter().map(Cow::from)
    }

    fn mood(&self) -> Option<Cow<'_, str>> {
        self.mood.as_deref().map(Cow::from)
    }

    fn movement(&self) -> Option<Cow<'_, str>> {
        self.movement.as_deref().map(Cow::from)
    }

    fn movement_count(&self) -> Option<Cow<'_, str>> {
        self.movement_count.as_deref().map(Cow::from)
    }

    fn movement_number(&self) -> Option<Cow<'_, str>> {
        self.movement_number.as_deref().map(Cow::from)
    }

    fn musicbrainz_artist_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_artist_id.as_deref().map(Cow::from)
    }

    fn musicbrainz_original_artist_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_original_artist_id
            .as_deref()
            .map(Cow::from)
    }

    fn musicbrainz_original_release_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_original_release_id
            .as_deref()
            .map(Cow::from)
    }

    fn musicbrainz_recording_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_recording_id.as_deref().map(Cow::from)
    }

    fn musicbrainz_track_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_track_id.as_deref().map(Cow::from)
    }

    fn musicbrainz_trm_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_trm_id.as_deref().map(Cow::from)
    }

    fn musicbrainz_work_id(&self) -> Option<Cow<'_, str>> {
        self.musicbrainz_work_id.as_deref().map(Cow::from)
    }

    fn musicip_fingerprint(&self) -> Option<Cow<'_, str>> {
        self.musicip_fingerprint.as_deref().map(Cow::from)
    }

    fn musicip_puid(&self) -> Option<Cow<'_, str>> {
        self.musicip_puid.as_deref().map(Cow::from)
    }

    fn original_album(&self) -> Option<Cow<'_, str>> {
        self.original_album.as_deref().map(Cow::from)
    }

    fn original_artist(&self) -> Option<Cow<'_, str>> {
        self.original_artist.as_deref().map(Cow::from)
    }

    fn original_filename(&self) -> Option<Cow<'_, str>> {
        self.original_filename.as_deref().map(Cow::from)
    }

    fn original_release_date(&self) -> Option<Cow<'_, str>> {
        self.original_release_date.as_deref().map(Cow::from)
    }

    fn original_release_year(&self) -> Option<Cow<'_, str>> {
        self.original_release_year.as_deref().map(Cow::from)
    }

    fn performers(&self) -> Option<Vec<InvolvedPerson<'_>>> {
        self.performers.as_deref().map(|slice| {
            slice
                .iter()
                .map(|(involvement, involvee)| InvolvedPerson {
                    involvement: Cow::from(involvement),
                    involvee: Cow::from(involvee),
                })
                .collect()
        })
    }

    fn producer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.producer.iter().map(Cow::from)
    }

    fn rating(&self) -> Option<Cow<'_, str>> {
        self.rating.as_deref().map(Cow::from)
    }

    fn remixer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.remixer.iter().map(Cow::from)
    }

    fn replay_gain_album_gain(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_album_gain.as_deref().map(Cow::from)
    }

    fn replay_gain_album_peak(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_album_peak.as_deref().map(Cow::from)
    }

    fn replay_gain_album_range(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_album_range.as_deref().map(Cow::from)
    }

    fn replay_gain_reference_loudness(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_reference_loudness
            .as_deref()
            .map(Cow::from)
    }

    fn replay_gain_track_gain(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_track_gain.as_deref().map(Cow::from)
    }

    fn replay_gain_track_peak(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_track_peak.as_deref().map(Cow::from)
    }

    fn replay_gain_track_range(&self) -> Option<Cow<'_, str>> {
        self.replay_gain_track_range.as_deref().map(Cow::from)
    }

    fn track_number(&self) -> Option<Cow<'_, str>> {
        self.track_number.as_deref().map(Cow::from)
    }

    fn track_title(&self) -> Option<Cow<'_, str>> {
        self.track_title.as_deref().map(Cow::from)
    }

    fn track_title_sort_order(&self) -> Option<Cow<'_, str>> {
        self.track_title_sort_order.as_deref().map(Cow::from)
    }

    fn artist_website(&self) -> Option<Cow<'_, str>> {
        self.artist_website.as_deref().map(Cow::from)
    }

    fn work_title(&self) -> Option<Cow<'_, str>> {
        self.work_title.as_deref().map(Cow::from)
    }

    fn writer(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.writer.iter().map(Cow::from)
    }

    fn track_length(&self) -> Option<chrono::TimeDelta> {
        self.track_length
            .and_then(|(secs, subsec_nanos)| chrono::TimeDelta::new(secs, subsec_nanos))
    }
}
