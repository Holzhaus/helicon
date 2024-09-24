// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! MusicBrainz helper functions.

use crate::distance::{Distance, DistanceItem, ReleaseSimilarity};
use crate::release::ReleaseLike;
use crate::Cache;
use crate::Config;
use futures::{
    future::TryFutureExt,
    stream::{self, StreamExt},
};
use musicbrainz_rs_nova::{
    entity::release::{
        Release as MusicBrainzRelease, ReleaseSearchQuery as MusicBrainzReleaseSearchQuery,
    },
    Fetch, Search,
};
use std::borrow::Borrow;
use std::collections::BinaryHeap;

/// A candidate release that potentially matches the base release.
pub struct ReleaseCandidate {
    /// The release from MusicBrainz.
    release: MusicBrainzRelease,
    /// The similarity to the base release.
    similarity: ReleaseSimilarity,
}

impl ReleaseCandidate {
    /// Create a new candidate from a musicbrainz release and it's similarity to the base release.
    pub fn new(release: MusicBrainzRelease, similarity: ReleaseSimilarity) -> Self {
        Self {
            release,
            similarity,
        }
    }

    /// Get a reference to the inner release;
    pub fn release(&self) -> &MusicBrainzRelease {
        &self.release
    }

    /// Get the distance to the base release.
    pub fn distance(&self) -> Distance {
        self.similarity.total_distance()
    }
}

/// Find MusicBrainz Release information for the given (generic) Release.
pub async fn find_releases(
    config: &Config,
    cache: Option<&impl Cache>,
    base_release: &impl ReleaseLike,
) -> crate::Result<Vec<ReleaseCandidate>> {
    if let Some(mb_id) = base_release.musicbrainz_release_id() {
        let release = find_release_by_mb_id(mb_id.into_owned(), cache).await?;
        let similarity = base_release.similarity_to(&release, config);
        let candidate = ReleaseCandidate::new(release, similarity);
        return Ok(vec![candidate]);
    }

    debug_assert_ne!(
        config.lookup.release_candidate_limit, None,
        "release_candidate_limit not configured!"
    );
    let max_candidate_count = config.lookup.release_candidate_limit.unwrap_or(25);
    let similar_release_ids =
        find_release_ids_by_similarity(cache, base_release, max_candidate_count, 0).await?;
    let heap = BinaryHeap::with_capacity(similar_release_ids.len());
    let heap = stream::iter(similar_release_ids)
        .map(|mb_id| find_release_by_mb_id(mb_id, cache))
        .buffer_unordered(config.lookup.connection_limit.unwrap_or(1))
        .fold(heap, |mut heap, result| async {
            let Ok(release) = result else {
                return heap;
            };

            let similarity = base_release.similarity_to(&release, config);
            let candidate = ReleaseCandidate::new(release, similarity);
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

    let releases: Vec<ReleaseCandidate> = heap
        .into_sorted_vec()
        .into_iter()
        .map(|dist_item: DistanceItem<ReleaseCandidate>| dist_item.item)
        .collect();
    log::info!("Found {} release candidates.", releases.len());
    Ok(releases)
}

/// Search for similar releases based on the metadata of an existing [`ReleaseLike`].
pub async fn find_release_ids_by_similarity(
    cache: Option<&impl Cache>,
    base_release: &impl ReleaseLike,
    limit: u8,
    offset: u16,
) -> crate::Result<Vec<String>> {
    let mut query = MusicBrainzReleaseSearchQuery::query_builder();
    let mut query = query.tracks(
        &base_release
            .track_count()
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
    let response = if let Some(cached_response) = cache.and_then(|c| c.get_release_search_result(&search_query, limit, offset)
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
        if let Some(c) = cache {
            match c.insert_release_search_result(&search_query, limit, offset, &response) {
                Ok(()) => {
                    log::debug!("Inserted release search {search_query:?} (limit: {limit}, offset: {offset}) into cache");
                }
                Err(err) => {
                    log::warn!("Failed to insert release search {search_query:?} (limit: {limit}, offset: {offset}) into cache: {err}");
                }
            };
        };
        response
    };

    let mb_ids = response
        .entities
        .into_iter()
        .map(|release| release.id)
        .collect();
    Ok(mb_ids)
}

/// Fetch a MusicBrainz release by its release ID.
pub async fn find_release_by_mb_id(
    id: String,
    cache: Option<&impl Cache>,
) -> crate::Result<MusicBrainzRelease> {
    if let Some(release) = cache.and_then(|c| {
        c.get_release(&id)
            .inspect_err(|err| {
                log::debug!("Failed to get release {id} from cache: {err}");
            })
            .ok()
    }) {
        return Ok(release);
    }

    MusicBrainzRelease::fetch()
        .id(&id)
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
            if let Some(c) = cache {
                match c.insert_release(&id, release) {
                    Ok(()) => {
                        log::debug!("Inserted release {id} into cache");
                    }
                    Err(err) => {
                        log::warn!("Failed to insert release {id} into cache: {err}");
                    }
                };
            };
        })
}

#[cfg(test)]
mod tests {
    use crate::release::ReleaseLike;
    use crate::track::TrackLike;
    use musicbrainz_rs_nova::entity::release::Release as MusicBrainzRelease;

    const MUSICBRAINZ_RELEASE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/musicbrainz/release.json"
    ));

    #[test]
    fn test_releaselike_impl() {
        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();

        assert_eq!(
            release.release_title().unwrap(),
            "Ahmad Jamal at the Pershing: But Not for Me"
        );
        assert_eq!(release.release_artist().unwrap(), "The Ahmad Jamal Trio");
        assert_eq!(release.track_count().unwrap(), 8);
        assert_eq!(
            release.musicbrainz_release_id().unwrap(),
            "0008f765-032b-46cd-ab69-2220edab1837"
        );
        assert_eq!(release.media_format().unwrap(), "12\" Vinyl");
        assert_eq!(release.record_label().unwrap(), "Argo");
        assert_eq!(release.catalog_number().unwrap(), "LP-628");
        assert_eq!(release.barcode(), None);
    }

    #[test]
    fn test_tracklike_impl() {
        let release: MusicBrainzRelease = serde_json::from_str(MUSICBRAINZ_RELEASE_JSON).unwrap();
        let track = release.tracks().skip(5).take(1).next().unwrap();

        assert_eq!(track.track_title().unwrap(), "Poinciana");
        assert_eq!(track.track_artist().unwrap(), "Ahmad Jamal");
        assert_eq!(track.track_number().unwrap(), "6");
        assert_eq!(
            track.track_length().unwrap(),
            chrono::TimeDelta::milliseconds(487_533)
        );
    }
}
