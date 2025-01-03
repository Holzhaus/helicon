// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Reusable utilities for the UI.

use crate::distance::Distance;
use crate::release::ReleaseLike;
use crossterm::style::{Color, Stylize};
use std::borrow::Cow;

mod layout;
mod styled_content;

pub use layout::{print_column_layout, LayoutItem};
pub use styled_content::{string_diff_opt, StyledContentList};

/// Format a distance as a similarity in percent, were 0% the the maximum distance and 100% the
/// minimum distance.
pub fn as_similarity_percentage(distance: &Distance) -> f64 {
    (1.0 - distance.as_f64()) * 100.0
}

/// Get the color associate with the distance value.
pub fn distance_color(distance: &Distance) -> Color {
    let d = distance.as_f64();
    if d <= 0.1 {
        Color::Green
    } else if d <= 0.5 {
        Color::Yellow
    } else {
        Color::Red
    }
}

/// Format the similarity as colored percentage.
pub fn format_similarity(distance: &Distance) -> String {
    let similarity = as_similarity_percentage(distance);
    let color = distance_color(distance);

    format!("{similarity:.02}").with(color).to_string()
}

/// Format the release artist and title for the terminal.
pub fn format_release_artist_and_title(release: &impl ReleaseLike) -> String {
    let artist = release
        .release_artist()
        .unwrap_or_else(|| Cow::from("[unknown artist]".grey().to_string()));
    let album = release
        .release_title()
        .unwrap_or_else(|| Cow::from("[unknown album]".grey().to_string()));

    format!("{artist} - {album}")
}
