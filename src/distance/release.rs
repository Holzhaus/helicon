// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions for distance calculation between [`Release`] objects.

use super::{Distance, DistanceBetween};
use crate::release::Release;
use std::borrow::Borrow;

/// Calculate the distance between two releases.
pub fn between<T1, T2>(lhs: &T1, rhs: &T2) -> Distance
where
    T1: Release + ?Sized,
    T2: Release + ?Sized,
{
    let release_title_distance =
        Distance::between(lhs.release_title(), rhs.release_title()).with_weight(3.0);
    let release_artist_distance =
        Distance::between(lhs.release_artist(), rhs.release_artist()).with_weight(3.0);
    let musicbrainz_release_id_distance = Distance::from(
        lhs.musicbrainz_release_id()
            .and_then(|lhs_id| rhs.musicbrainz_release_id().map(|rhs_id| (lhs_id, rhs_id)))
            .is_some_and(|(lhs, rhs)| {
                let lhs_id: &str = lhs.borrow();
                let lhs_id: &str = lhs_id.trim();

                let rhs_id: &str = rhs.borrow();
                let rhs_id: &str = rhs_id.trim();

                lhs_id == rhs_id && !lhs_id.is_empty()
            }),
    )
    .with_weight(5.0);
    let media_format_distance =
        Distance::between(lhs.media_format(), rhs.media_format()).with_weight(1.0);
    let record_label_distance =
        Distance::between(lhs.record_label(), rhs.record_label()).with_weight(0.5);
    let catalog_number_distance =
        Distance::between(lhs.catalog_number(), rhs.catalog_number()).with_weight(0.5);
    let barcode_distance = Distance::between(lhs.barcode(), rhs.barcode()).with_weight(0.5);

    let distances = [
        release_title_distance,
        release_artist_distance,
        musicbrainz_release_id_distance,
        media_format_distance,
        record_label_distance,
        catalog_number_distance,
        barcode_distance,
    ];
    Distance::from(distances.as_slice())
}
