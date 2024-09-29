// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Layout utilities.

use super::styled_content::CharWidth;
use super::StyledContentList;
use crossterm::style::{ContentStyle, StyledContent};
use std::borrow::Cow;

/// An item that can be passed to `print_column_layout`.
pub struct LayoutItem<'a> {
    /// The prefix that will be displayed on the first line.
    pub prefix: StyledContentList<'a>,
    /// The content that might be longer than a single line.
    pub content: StyledContentList<'a>,
    /// The suffix that will be displayed on the first line.
    pub suffix: StyledContentList<'a>,
}

impl CharWidth for LayoutItem<'_> {
    fn char_width(&self) -> usize {
        self.prefix.char_width() + self.content.char_width() + self.suffix.char_width()
    }

    fn byte_count(&self) -> usize {
        self.prefix.byte_count() + self.content.byte_count() + self.suffix.byte_count()
    }
}

impl<'a> LayoutItem<'a> {
    /// Create a new layout item that uses an existing [`StyledContentList`] as content.
    pub fn new(content: StyledContentList<'a>) -> Self {
        LayoutItem {
            prefix: StyledContentList::default(),
            content,
            suffix: StyledContentList::default(),
        }
    }

    /// Set this item's prefix.
    pub fn with_prefix(mut self, value: StyledContentList<'a>) -> Self {
        self.prefix = value;
        self
    }

    /// Set this item's suffix.
    pub fn with_suffix(mut self, value: StyledContentList<'a>) -> Self {
        self.suffix = value;
        self
    }

    /// Returns `true` if the unstyled prefix, content and suffix are all empty.
    #[expect(dead_code)]
    pub fn is_empty(&'a self) -> bool {
        self.char_width() == 0
    }

    /// Split this layout item into multiple lines, each line not longer than `max_width`.
    /// Prefix and suffix will be placed on the first line.
    fn into_split_lines(mut self, max_width: usize) -> impl Iterator<Item = StyledContentList<'a>> {
        let first_line_content_width =
            max_width - self.prefix.char_width() - self.suffix.char_width();
        let second = self.content.split_off(first_line_content_width);
        debug_assert!(self.char_width() <= max_width);
        debug_assert!(self.content.char_width() <= first_line_content_width);
        self.content = self.content.fill_right(' ', first_line_content_width);
        debug_assert!(self.content.char_width() <= first_line_content_width);
        debug_assert_eq!(self.char_width(), max_width);
        let first: StyledContentList<'a> = self.into();
        [first].into_iter().chain([second].into_iter().flatten())
    }
}

/// Print two layout items in a two column layout.
///
/// Each layout items contains of of a prefix, the content, and the suffix.
///
///
/// # Example
/// ## Single Line
///
/// If both sides fit into a single row and thus do not require linebreaks, the output will look
/// like this.
///
/// ```text
/// [indent][lhs_prefix][ lhs_content ][lhs_suffix][separator][rhs_prefix][ rhs_content ][rhs_suffix]
/// ```
///
/// ## Multi-Line
///
/// If both the left-hand side and the right-hand side require multiple lines, the content will
/// be rendered like this:
///
/// ```text
/// [indent][lhs_prefix][lhs_content..][lhs_suffix][separator][rhs_prefix][rhs_content..][rhs_suffix]
///         [         ..lhs_content..             ]           [         ..rhs_content..             ]
///         [          ..lhs_content              ]           [          ..rhs_content              ]
/// ```
pub fn print_column_layout(
    lhs: LayoutItem<'_>,
    rhs: LayoutItem<'_>,
    indent: &str,
    separator: &str,
    max_width: usize,
) {
    let column_width = (max_width - indent.len() - separator.len()) / 2;
    let mut lhs_lines = lhs.into_split_lines(column_width);
    let mut rhs_lines = rhs.into_split_lines(column_width);

    let lhs_line: Option<StyledContentList<'_>> = lhs_lines.next();
    let rhs_line: Option<StyledContentList<'_>> = rhs_lines.next();
    let mut next_line: Option<StyledContentList<'_>> =
        [StyledContent::new(ContentStyle::new(), Cow::from(indent))]
            .into_iter()
            .chain(
                lhs_line
                    .into_iter()
                    .map(|line| line.fill_right(' ', column_width))
                    .flat_map(StyledContentList::into_iter),
            )
            .chain([StyledContent::new(
                ContentStyle::new(),
                Cow::from(separator),
            )])
            .chain(rhs_line.into_iter().flat_map(StyledContentList::into_iter))
            .collect::<StyledContentList<'_>>()
            .into();

    while let Some(line) = next_line {
        println!("{line}");

        let lhs_line: Option<StyledContentList<'_>> = lhs_lines.next();
        let rhs_line: Option<StyledContentList<'_>> = rhs_lines.next();

        if lhs_line.is_some() && rhs_line.is_some() {
            next_line = StyledContentList::default()
                .fill_right(' ', indent.len())
                .into_iter()
                .chain(
                    lhs_line
                        .into_iter()
                        .map(|line| line.fill_right(' ', column_width))
                        .flat_map(StyledContentList::into_iter),
                )
                .chain(
                    StyledContentList::default()
                        .fill_right(' ', separator.len())
                        .into_iter(),
                )
                .chain(rhs_line.into_iter().flat_map(StyledContentList::into_iter))
                .collect::<StyledContentList<'_>>()
                .into();
        } else {
            next_line = None;
        }
    }
}
