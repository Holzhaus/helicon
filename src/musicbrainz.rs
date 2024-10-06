// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! MusicBrainz helper functions.

use crate::distance::DistanceItem;
use crate::release::ReleaseLike;
use crate::release_candidate::ReleaseCandidate;
use crate::Cache;
use crate::Config;
use futures::{
    future::TryFutureExt,
    stream::{self, Stream, StreamExt},
};
pub use musicbrainz_rs_nova::entity::{
    release::Release as MusicBrainzRelease, release_group::ReleaseGroup as MusicBrainzReleaseGroup,
};
use musicbrainz_rs_nova::{
    entity::release::ReleaseSearchQuery as MusicBrainzReleaseSearchQuery, Fetch, Search,
};
use regex::Regex;
use std::borrow::{Borrow, Cow};
use std::collections::BinaryHeap;

/// Configurable MusicBrainz API client with caching support.
#[derive(Debug)]
pub struct MusicBrainzClient<'a> {
    /// Configuration
    config: &'a Config,
    /// Cache
    cache: Option<&'a Cache>,
}

impl<'a> MusicBrainzClient<'a> {
    /// Create a new MusicBrainz client.
    pub fn new(config: &'a Config, cache: Option<&'a Cache>) -> Self {
        Self { config, cache }
    }

    /// Find MusicBrainz Release information for the given (generic) Release.
    pub async fn find_releases_by_similarity(
        &self,
        base_release: &impl ReleaseLike,
    ) -> crate::Result<Vec<ReleaseCandidate<MusicBrainzRelease>>> {
        if let Some(release_id) = base_release.musicbrainz_release_id() {
            let release = self.find_release_by_id(release_id.into_owned()).await?;
            let candidate =
                ReleaseCandidate::new_with_base_release(release, base_release, self.config);
            return Ok(vec![candidate]);
        }

        let similar_release_ids = self
            .find_release_ids_by_similarity(
                base_release,
                self.config.lookup.release_candidate_limit,
                0,
            )
            .await?;
        let heap = BinaryHeap::with_capacity(similar_release_ids.len());
        let heap = stream::iter(similar_release_ids)
            .map(|release_id| self.find_release_by_id(release_id))
            .buffer_unordered(self.config.lookup.connection_limit)
            .fold(heap, |mut heap, result| async {
                let Ok(release) = result else {
                    return heap;
                };

                let candidate =
                    ReleaseCandidate::new_with_base_release(release, base_release, self.config);
                let candidate_distance = candidate.distance();

                log::debug!(
                    "Release '{}' has distance to track collection: {}",
                    candidate.release().title,
                    candidate_distance.weighted_distance()
                );
                let item = DistanceItem::new(candidate, candidate_distance);
                heap.push(item);
                heap
            })
            .await;

        let releases: Vec<ReleaseCandidate<MusicBrainzRelease>> = heap
            .into_sorted_vec()
            .into_iter()
            .map(|dist_item: DistanceItem<ReleaseCandidate<MusicBrainzRelease>>| dist_item.item)
            .collect();
        log::info!("Found {} release candidates.", releases.len());
        Ok(releases)
    }

    /// Search for similar releases based on the metadata of an existing [`ReleaseLike`].
    async fn find_release_ids_by_similarity(
        &self,
        base_release: &impl ReleaseLike,
        limit: u8,
        offset: u16,
    ) -> crate::Result<Vec<String>> {
        let mut query = MusicBrainzReleaseSearchQuery::query_builder();
        let mut query = query.tracks(
            &base_release
                .release_track_count()
                .map(|track_count| track_count.to_string())
                .unwrap_or_default(),
        );
        if let Some(v) = base_release.release_artist() {
            query = query.and().artist(v.borrow());
        };
        if let Some(v) = base_release.release_title() {
            query = query.and().release(v.borrow());
        };
        if let Some(v) = base_release.catalog_number() {
            query = query.and().catalog_number(v.borrow());
        };
        if let Some(v) = base_release.barcode() {
            query = query.and().barcode(v.borrow());
        }

        let search_query = query.build();
        let response = if let Some(cached_response) = self.cache.and_then(|cache| cache.get_item((search_query.as_ref(), limit, offset))
                .inspect_err(|err| {
                    log::debug!("Failed to get release search result for query {search_query} (limit {limit}) from cache: {err}");
                })
                .ok()) {
            cached_response
        } else {
            let response = MusicBrainzRelease::search(search_query.clone())
                .limit(limit)
                .offset(offset)
                .execute()
                .await?;
            log::debug!(
                "Found {} releases using query: {}",
                response.entities.len(),
                search_query
            );
            if let Some(cache) = self.cache {
                match cache.insert_item((search_query.as_ref(), limit, offset), &response) {
                Ok(()) => {
                    log::debug!("Inserted release search {search_query:?} (limit: {limit}, offset: {offset}) into cache");
                }
                Err(err) => {
                    log::warn!("Failed to insert release search {search_query:?} (limit: {limit}, offset: {offset}) into cache: {err}");
                }
            }
            };
            response
        };

        let ids = response
            .entities
            .into_iter()
            .map(|release| release.id)
            .collect();
        Ok(ids)
    }

    /// Fetch a MusicBrainz release group by its ID.
    async fn find_release_group_by_id(
        &self,
        release_group_id: String,
    ) -> crate::Result<MusicBrainzReleaseGroup> {
        if let Some(release_group) = self.cache.and_then(|cache| {
            cache
                .get_item(release_group_id.as_ref())
                .inspect_err(|err| {
                    log::debug!("Failed to get release_group {release_group_id} from cache: {err}");
                })
                .ok()
        }) {
            return Ok(release_group);
        }

        MusicBrainzReleaseGroup::fetch()
            .id(&release_group_id)
            .with_releases()
            .execute()
            .map_err(crate::Error::from)
            .await
            .inspect(|release_group| {
                if let Some(cache) = self.cache {
                    match cache.insert_item(release_group_id.as_ref(), release_group) {
                        Ok(()) => {
                            log::debug!("Inserted release group {release_group_id} into cache");
                        }
                        Err(err) => {
                            log::warn!("Failed to insert release group {release_group_id} into cache: {err}");
                        }
                    }};
            })
    }

    /// Find release IDs by MusicBrainz Release Group ID.
    async fn find_release_ids_by_release_group_id(
        &self,
        release_group_id: String,
    ) -> crate::Result<Vec<String>> {
        let release_group = self.find_release_group_by_id(release_group_id).await?;
        let Some(releases) = release_group.releases else {
            log::warn!("Release group has no releases!");
            return Err(crate::Error::MusicBrainzLookupFailed(
                "Release Group has no releases.",
            ));
        };

        let release_ids = releases.into_iter().map(|release| release.id).collect();
        Ok(release_ids)
    }

    /// Find releases by MusicBrainz Release Group ID.
    pub async fn find_releases_by_release_group_id(
        &self,
        release_group_id: String,
    ) -> crate::Result<impl Stream<Item = crate::Result<MusicBrainzRelease>> + '_> {
        let release_ids = self
            .find_release_ids_by_release_group_id(release_group_id)
            .await?;
        let release_stream = stream::iter(release_ids)
            .map(move |release_id| self.find_release_by_id(release_id))
            .buffer_unordered(self.config.lookup.connection_limit);
        Ok(release_stream)
    }

    /// Fetch a MusicBrainz release by its release ID.
    pub async fn find_release_by_id(
        &self,
        release_id: String,
    ) -> crate::Result<MusicBrainzRelease> {
        if let Some(release) = self.cache.and_then(|cache| {
            cache
                .get_item(release_id.as_ref())
                .inspect_err(|err| {
                    log::debug!("Failed to get release {release_id} from cache: {err}");
                })
                .ok()
        }) {
            return Ok(release);
        }

        MusicBrainzRelease::fetch()
            .id(&release_id)
            .with_artists()
            .with_recordings()
            .with_release_groups()
            .with_labels()
            .with_artist_credits()
            .with_aliases()
            .with_recording_level_relations()
            .with_work_relations()
            .with_work_level_relations()
            .with_artist_relations()
            .with_url_relations()
            .execute()
            .map_err(crate::Error::from)
            .await
            .inspect(|release| {
                if let Some(cache) = self.cache {
                    match cache.insert_item(release_id.as_ref(), release) {
                        Ok(()) => {
                            log::debug!("Inserted release {release_id} into cache");
                        }
                        Err(err) => {
                            log::warn!("Failed to insert release {release_id} into cache: {err}");
                        }
                    }
                };
            })
    }
}

/// A MusicBrainz Identifier.
///
/// See <https://musicbrainz.org/doc/MusicBrainz_Identifier> for details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicBrainzId<'a> {
    /// An area ID.
    ///
    /// See <https://musicbrainz.org/doc/Area> for details.
    Area(Cow<'a, str>),
    /// An artist ID.
    ///
    /// See <https://musicbrainz.org/doc/Artist> for details.
    Artist(Cow<'a, str>),
    /// An event ID.
    ///
    /// See <https://musicbrainz.org/doc/Event> for details.
    Event(Cow<'a, str>),
    /// A genre ID.
    ///
    /// See <https://musicbrainz.org/doc/Genre> for details.
    Genre(Cow<'a, str>),
    /// A instrument ID.
    ///
    /// See <https://musicbrainz.org/doc/Instrument> for details.
    Instrument(Cow<'a, str>),
    /// A record label ID.
    ///
    /// See <https://musicbrainz.org/doc/Label> for details.
    Label(Cow<'a, str>),
    /// A place ID.
    ///
    /// See <https://musicbrainz.org/doc/Place> for details.
    Place(Cow<'a, str>),
    /// A recording ID.
    ///
    /// See <https://musicbrainz.org/doc/Recording> for details.
    Recording(Cow<'a, str>),
    /// A release ID.
    ///
    /// See <https://musicbrainz.org/doc/Release> for details.
    Release(Cow<'a, str>),
    /// A release group ID.
    ///
    /// See <https://musicbrainz.org/doc/Release_Group> for details.
    ReleaseGroup(Cow<'a, str>),
    /// A series ID.
    ///
    /// See <https://musicbrainz.org/doc/Series> for details.
    Series(Cow<'a, str>),
    /// A work ID.
    ///
    /// See <https://musicbrainz.org/doc/Work> for details.
    Work(Cow<'a, str>),
}

impl<'a> MusicBrainzId<'a> {
    /// Get the entity name for the given ID as string.
    pub fn entity_name(&self) -> &'static str {
        match &self {
            Self::Area(_) => "area",
            Self::Artist(_) => "artist",
            Self::Event(_) => "event",
            Self::Genre(_) => "genre",
            Self::Instrument(_) => "instrument",
            Self::Label(_) => "label",
            Self::Place(_) => "place",
            Self::Recording(_) => "recording",
            Self::Release(_) => "release",
            Self::ReleaseGroup(_) => "release-group",
            Self::Series(_) => "series",
            Self::Work(_) => "work",
        }
    }

    /// Find a MusicBrainz ID in a string.
    ///
    /// If the input contains an ID directly, a release ID is assumed.
    pub fn find(input: &'a str) -> Option<Self> {
        let re = Regex::new(
            r"\b[0-9a-fA-F]{8}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{12}\b",
        )
        .ok()?;
        if let Some(m) = re.find(input) {
            if m.start() == 0 {
                return Some(Self::Release(m.as_str().into()));
            }

            if let Some(pos) = input[..m.start() - 1].rfind('/') {
                let is_valid_url = [
                    "http://musicbrainz.org/",
                    "https://musicbrainz.org/",
                    "http://musicbrainz.org/ws/2/",
                    "https://musicbrainz.org/ws/2/",
                ]
                .into_iter()
                .any(|x| x == &input[..=pos]);
                if is_valid_url {
                    let entity_name = &input[pos + 1..m.start() - 1];
                    return match entity_name {
                        "area" => Self::Area(m.as_str().into()).into(),
                        "artist" => Self::Artist(m.as_str().into()).into(),
                        "event" => Self::Event(m.as_str().into()).into(),
                        "genre" => Self::Genre(m.as_str().into()).into(),
                        "instrument" => Self::Instrument(m.as_str().into()).into(),
                        "label" => Self::Label(m.as_str().into()).into(),
                        "place" => Self::Place(m.as_str().into()).into(),
                        "recording" => Self::Recording(m.as_str().into()).into(),
                        "release" => Self::Release(m.as_str().into()).into(),
                        "release-group" => Self::ReleaseGroup(m.as_str().into()).into(),
                        "series" => Self::Series(m.as_str().into()).into(),
                        "work" => Self::Work(m.as_str().into()).into(),
                        _ => None,
                    };
                }
            }
        };
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::ReleaseLike;
    use crate::track::TrackLike;
    use musicbrainz_rs_nova::entity::release::Release as MusicBrainzRelease;

    const MUSICBRAINZ_RELEASE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/musicbrainz/release.json"
    ));

    #[test]
    fn test_find_musicbrainz_id() {
        assert_eq!(
            MusicBrainzId::find("0008f765-032b-46cd-ab69-2220edab1837"),
            Some(MusicBrainzId::Release(
                "0008f765-032b-46cd-ab69-2220edab1837".into()
            ))
        );
        assert_eq!(
            MusicBrainzId::find(
                "https://musicbrainz.org/release/0008f765-032b-46cd-ab69-2220edab1837"
            ),
            Some(MusicBrainzId::Release(
                "0008f765-032b-46cd-ab69-2220edab1837".into()
            ))
        );
        assert_eq!(
            MusicBrainzId::find(
                "http://musicbrainz.org/release/0008f765-032b-46cd-ab69-2220edab1837"
            ),
            Some(MusicBrainzId::Release(
                "0008f765-032b-46cd-ab69-2220edab1837".into()
            ))
        );
        assert_eq!(MusicBrainzId::find("http://musicbrainz.org/ws/2/release/0008f765-032b-46cd-ab69-2220edab1837?inc=artists%20recordings%20release-groups"), Some(MusicBrainzId::Release("0008f765-032b-46cd-ab69-2220edab1837".into())));
        assert_eq!(
            MusicBrainzId::find(
                "https://musicbrainz.org/recording/9d444787-3f25-4c16-9261-597b9ab021cc"
            ),
            Some(MusicBrainzId::Recording(
                "9d444787-3f25-4c16-9261-597b9ab021cc".into()
            ))
        );
        assert_eq!(
            MusicBrainzId::find(
                "https://musicbrainz.org/release-group/0a8e97fd-457c-30bc-938a-2fba79cb04e7"
            ),
            Some(MusicBrainzId::ReleaseGroup(
                "0a8e97fd-457c-30bc-938a-2fba79cb04e7".into()
            ))
        );
        assert_eq!(MusicBrainzId::find("some random string"), None);
    }

    #[test]
    fn test_releaselike_impl() {
        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();

        assert_eq!(
            release.release_title().unwrap(),
            "Ahmad Jamal at the Pershing: But Not for Me"
        );
        assert_eq!(release.release_artist().unwrap(), "The Ahmad Jamal Trio");
        assert_eq!(release.release_track_count().unwrap(), 8);
        assert_eq!(
            release.musicbrainz_release_id().unwrap(),
            "0008f765-032b-46cd-ab69-2220edab1837"
        );
        assert_eq!(release.release_media_format().unwrap(), "12\" Vinyl");
        assert_eq!(release.record_label().unwrap(), "Argo");
        assert_eq!(release.catalog_number().unwrap(), "LP-628");
        assert_eq!(release.barcode(), None);
    }

    #[test]
    fn test_tracklike_impl() {
        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let track = release.release_tracks().skip(5).take(1).next().unwrap();

        assert_eq!(track.track_title().unwrap(), "Poinciana");
        assert_eq!(track.track_artist().unwrap(), "The Ahmad Jamal Trio");
        assert_eq!(track.track_number().unwrap(), "6");
        assert_eq!(
            track.track_length().unwrap(),
            chrono::TimeDelta::milliseconds(487_533)
        );
    }
}
