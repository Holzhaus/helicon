// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! User Interface (UI) utilities.

mod handle_candidate;
mod select_candidate;
mod util;

pub use handle_candidate::{handle_candidate, HandleCandidateResult};
pub use select_candidate::{select_candidate, ReleaseCandidateSelectionResult};
