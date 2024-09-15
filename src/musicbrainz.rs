// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! MusicBrainz helper functions.

use crate::release::ReleaseLike;
use futures::{
    future::{self, FutureExt},
    stream::{self, StreamExt},
    Stream,
};
use musicbrainz_rs_nova::{
    entity::release::{
        Release as MusicBrainzRelease, ReleaseSearchQuery as MusicBrainzReleaseSearchQuery,
    },
    Fetch, Search,
};
use std::borrow::Borrow;

/// Find MusicBrainz Release information for the given (generic) Release.
pub fn find_releases(
    base_release: &impl ReleaseLike,
) -> impl Stream<Item = crate::Result<MusicBrainzRelease>> + '_ {
    base_release
        .musicbrainz_release_id()
        .inspect(|mb_release_id| {
            log::info!("Found MusicBrainz Release Id: {:?}", mb_release_id);
        })
        .map_or_else(
            || future::ready(None).left_future(),
            |mb_id| {
                let mb_id = mb_id.to_string();
                async { find_release_by_mb_id(mb_id).await.ok() }.right_future()
            },
        )
        .map(move |result| {
            if let Some(release) = result {
                stream::once(future::ok(release)).left_stream()
            } else {
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

                let search = query.build();
                async { MusicBrainzRelease::search(search).execute().await }
                    .map(|result| {
                        result.map_or_else(
                            |_| stream::empty().left_stream(),
                            |response| stream::iter(response.entities).right_stream(),
                        )
                    })
                    .flatten_stream()
                    .map(|release| release.id)
                    .then(find_release_by_mb_id)
                    .right_stream()
            }
        })
        .into_stream()
        .flatten()
}

/// Fetch a MusicBrainz release by its release ID.
pub async fn find_release_by_mb_id(id: String) -> crate::Result<MusicBrainzRelease> {
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
        .map(|result| result.map_err(crate::Error::from))
        .await
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
