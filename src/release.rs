// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Generic release implementations.
use crate::distance::ReleaseDistance;
use musicbrainz_rs_nova::entity::release::Release as MusicBrainzRelease;

/// Represent a generic release, independent of the underlying source.
pub trait Release {
    /// Release title.
    fn release_title(&self) -> Option<&str>;
    /// Release artist.
    fn release_artist(&self) -> Option<&str>;

    /// Calculate the distance between this release and another one.
    fn distance_to<T>(&self, other: &T) -> ReleaseDistance
    where
        T: Release + ?Sized,
    {
        ReleaseDistance::between(self, other)
    }
}

impl Release for MusicBrainzRelease {
    fn release_title(&self) -> Option<&str> {
        Some(&self.title)
    }

    fn release_artist(&self) -> Option<&str> {
        None
    }
}
