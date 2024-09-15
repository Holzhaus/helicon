// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Functions for distance calculation between times.

use super::{Distance, DistanceBetween};
use chrono::TimeDelta;

/// Calculate the distance between two [`TimeDelta`] structs.
pub fn between(lhs: TimeDelta, rhs: TimeDelta) -> Distance {
    let grace = TimeDelta::seconds(10);
    let max_diff = TimeDelta::seconds(30);
    let diff = (lhs - rhs).abs();
    let normalized_diff = diff
        .checked_sub(&grace)
        .unwrap_or(TimeDelta::zero())
        .clamp(TimeDelta::zero(), max_diff);
    Distance::between(normalized_diff.num_seconds(), max_diff.num_seconds())
}
