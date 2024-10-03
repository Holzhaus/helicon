// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utilities for working with crossterm's `StyledContent`.

use super::LayoutItem;
use crate::config::StringDiffStyleConfig;
use crossterm::style::{ContentStyle, StyledContent};
use std::borrow::Cow;
use std::fmt;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Calculate a diff between the two strings, and return accordingly styled versions of the
/// strings.
pub fn string_diff(
    lhs: &str,
    rhs: &str,
    config: &StringDiffStyleConfig,
) -> (Vec<StyledContent<String>>, Vec<StyledContent<String>>) {
    let lhs_chars = lhs.chars().collect::<Vec<char>>();
    let rhs_chars = rhs.chars().collect::<Vec<char>>();

    let (lhs_diff, rhs_diff): (Vec<_>, Vec<_>) = similar::capture_diff(
        similar::Algorithm::Myers,
        &lhs_chars,
        0..lhs_chars.len(),
        &rhs_chars,
        0..rhs_chars.len(),
    )
    .into_iter()
    .map(|diffop| match diffop {
        similar::DiffOp::Equal {
            old_index,
            new_index,
            len,
        } => (
            Some(
                config.equal.apply(
                    lhs_chars[old_index..old_index + len]
                        .iter()
                        .collect::<String>(),
                ),
            ),
            Some(
                config.equal.apply(
                    rhs_chars[new_index..new_index + len]
                        .iter()
                        .collect::<String>(),
                ),
            ),
        ),
        similar::DiffOp::Delete {
            old_index, old_len, ..
        } => (
            Some(
                config.delete.apply(
                    lhs_chars[old_index..old_index + old_len]
                        .iter()
                        .collect::<String>(),
                ),
            ),
            None,
        ),
        similar::DiffOp::Insert {
            new_index, new_len, ..
        } => (
            None,
            Some(
                config.insert.apply(
                    rhs_chars[new_index..new_index + new_len]
                        .iter()
                        .collect::<String>(),
                ),
            ),
        ),
        similar::DiffOp::Replace {
            old_index,
            old_len,
            new_index,
            new_len,
        } => (
            Some(
                config.replace_old.apply(
                    lhs_chars[old_index..old_index + old_len]
                        .iter()
                        .collect::<String>(),
                ),
            ),
            Some(
                config.replace_new.apply(
                    rhs_chars[new_index..new_index + new_len]
                        .iter()
                        .collect::<String>(),
                ),
            ),
        ),
    })
    .unzip();
    (
        lhs_diff.into_iter().flatten().collect::<Vec<_>>(),
        rhs_diff.into_iter().flatten().collect::<Vec<_>>(),
    )
}

/// Similar to [`string_diff`], but also supports `None` values.
pub fn string_diff_opt<'a, 'b>(
    lhs: Option<Cow<'a, str>>,
    rhs: Option<Cow<'b, str>>,
    missing_value: &'static str,
    config: &StringDiffStyleConfig,
) -> (StyledContentList<'a>, StyledContentList<'b>) {
    match (lhs, rhs) {
        (Some(lhs_value), Some(rhs_value)) => {
            let (lhs_diff, rhs_diff) = string_diff(&lhs_value, &rhs_value, config);
            (
                StyledContentList::from(lhs_diff),
                StyledContentList::from(rhs_diff),
            )
        }
        (Some(lhs_track_title), None) => (
            config.present.apply(lhs_track_title).into(),
            config.missing.apply(Cow::from(missing_value)).into(),
        ),
        (None, Some(rhs_track_title)) => (
            config.missing.apply(Cow::from(missing_value)).into(),
            config.present.apply(rhs_track_title).into(),
        ),
        (None, None) => (
            config.missing.apply(Cow::from(missing_value)).into(),
            config.missing.apply(Cow::from(missing_value)).into(),
        ),
    }
}

/// Indicates that the visible char width and the number of bytes can be determined.
pub trait CharWidth {
    /// Count the number of bytes.
    ///
    /// For [`str`] this is equivalent to [`str.len()`].
    fn byte_count(&self) -> usize;
    /// Determine the visible width.
    fn char_width(&self) -> usize;
}

impl CharWidth for &str {
    fn byte_count(&self) -> usize {
        self.len()
    }

    fn char_width(&self) -> usize {
        self.width()
    }
}

impl CharWidth for Cow<'_, str> {
    fn byte_count(&self) -> usize {
        self.as_ref().len()
    }

    fn char_width(&self) -> usize {
        self.as_ref().width()
    }
}

impl CharWidth for StyledContent<Cow<'_, str>> {
    fn byte_count(&self) -> usize {
        self.content().byte_count()
    }

    fn char_width(&self) -> usize {
        self.content().char_width()
    }
}

impl CharWidth for StyledContentList<'_> {
    fn byte_count(&self) -> usize {
        self.0.iter().map(CharWidth::byte_count).sum()
    }

    fn char_width(&self) -> usize {
        self.0.iter().map(CharWidth::char_width).sum()
    }
}

/// Convert a [`StyledContent`] item into `StyledContent<Cow<'_, str>>`.
#[expect(clippy::needless_pass_by_value)]
pub fn convert_styled_content<'b, D: fmt::Display + Into<Cow<'b, str>> + Clone>(
    value: StyledContent<D>,
) -> StyledContent<Cow<'b, str>> {
    let style = *value.style();
    // FIXME: Remove this clone if https://github.com/crossterm-rs/crossterm/issues/932 is
    // fixed.
    let content = value.content().clone();
    StyledContent::new(style, content.into())
}

/// A list of [`StyledContent`] items.
#[derive(Debug, Default, Clone)]
pub struct StyledContentList<'a>(Vec<StyledContent<Cow<'a, str>>>);

impl fmt::Display for StyledContentList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in &self.0 {
            item.fmt(f)?;
        }
        Ok(())
    }
}

impl<'a> IntoIterator for StyledContentList<'a> {
    type Item = StyledContent<Cow<'a, str>>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> FromIterator<StyledContent<Cow<'a, str>>> for StyledContentList<'a> {
    fn from_iter<I: IntoIterator<Item = StyledContent<Cow<'a, str>>>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect::<Vec<_>>())
    }
}

impl<'a> From<StyledContent<&'a str>> for StyledContentList<'a> {
    fn from(value: StyledContent<&'a str>) -> Self {
        Self::new(vec![convert_styled_content(value)])
    }
}

impl<'a> From<StyledContent<Cow<'a, str>>> for StyledContentList<'a> {
    fn from(value: StyledContent<Cow<'a, str>>) -> Self {
        Self::new(vec![value])
    }
}

impl From<Vec<StyledContent<String>>> for StyledContentList<'_> {
    fn from(value: Vec<StyledContent<String>>) -> Self {
        value
            .into_iter()
            .map(convert_styled_content)
            .collect::<Self>()
    }
}

impl<'a> From<LayoutItem<'a>> for StyledContentList<'a> {
    fn from(value: LayoutItem<'a>) -> Self {
        value
            .prefix
            .into_iter()
            .chain(value.content)
            .chain(value.suffix)
            .collect::<StyledContentList<'a>>()
    }
}

impl<'a> StyledContentList<'a> {
    /// Create a new list from an existing vector.
    pub fn new(value: Vec<StyledContent<Cow<'a, str>>>) -> Self {
        Self(value)
    }

    /// Number of elements in this list.
    ///
    /// Only used in tests.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.0.len()
    }

    /// Split off the items after `max_width`. Modifies the original list and returns the content
    /// that was split off. Returns `None` if the length of its contents is shorter than
    /// `max_width`.
    pub fn split_off(&mut self, max_width: usize) -> Option<Self> {
        // No splitting necessary.
        if self.char_width() <= max_width {
            return None;
        }

        // Find the split position and the width if we split an item border.
        let (first_item_count, first_item_width) = self
            .0
            .iter()
            .map(|styled_content| styled_content.content().graphemes(true).count())
            .scan(0, |total_width, item_width| {
                *total_width += item_width;
                Some(*total_width)
            })
            .take_while(|total_width| *total_width <= max_width)
            .enumerate()
            .last()
            .map_or((0, 0), |(i, width)| (i + 1, width));
        debug_assert!(first_item_width <= max_width);

        // Split the two lists at the item border.
        let mut second_vec = self.0.split_off(first_item_count);
        debug_assert!(!second_vec.is_empty());
        debug_assert!(self.char_width() <= max_width);

        // Now check if we can split the first item in the second list and move the first part into
        // the the first list and keep the remainder in the second list.
        let remaining_width = max_width - first_item_width;
        let split_position_bytes = second_vec[0]
            .content()
            .graphemes(true)
            .scan((0usize, 0usize), |(byte_count, char_width), grapheme| {
                *byte_count += grapheme.byte_count();
                *char_width += grapheme.char_width();
                Some((*byte_count, *char_width))
            })
            .take_while(|(_, char_width)| *char_width <= remaining_width)
            .map(|(byte_count, _)| byte_count)
            .last();

        if let Some(byte_position) = split_position_bytes {
            let item = second_vec.remove(0);

            let original_width = item.char_width();

            // Split the item
            let (left_content, right_content) = item.content().split_at(byte_position);
            debug_assert_eq!(left_content.byte_count(), byte_position);
            debug_assert_eq!(left_content.char_width(), remaining_width);

            let left_item = item.style().apply(Cow::from(left_content.to_owned()));
            let right_item = item.style().apply(Cow::from(right_content.to_owned()));
            debug_assert_eq!(
                left_item.char_width() + right_item.char_width(),
                original_width
            );
            debug_assert_eq!(left_item.char_width(), remaining_width);

            // Insert the items into the list
            self.0.push(left_item);
            second_vec.insert(0, right_item);
        }
        debug_assert!(self.char_width() <= max_width);

        Some(Self::new(second_vec))
    }

    /// Append an item consisting of fill [`char`] to this list so that the length of its contents
    /// equals `desired_width`.
    pub fn fill_right(mut self, fill_char: char, desired_width: usize) -> Self {
        if self.char_width() >= desired_width {
            return self;
        }

        let missing_chars = desired_width - self.char_width();
        self.0.push(StyledContent::new(
            ContentStyle::new(),
            Cow::from(
                std::iter::repeat(fill_char)
                    .take(missing_chars)
                    .collect::<String>(),
            ),
        ));
        debug_assert_eq!(self.char_width(), desired_width);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Stylize;

    #[test]
    fn test_str_char_width_empty() {
        let text = "";
        assert_eq!(text.char_width(), 0);
    }

    #[test]
    fn test_str_byte_count_empty() {
        let text = "";
        assert_eq!(text.byte_count(), 0);
    }

    #[test]
    fn test_str_char_width_normal() {
        let text = "abcdef";
        assert_eq!(text.char_width(), 6);
    }

    #[test]
    fn test_str_byte_count_normal() {
        let text = "abcdef";
        assert_eq!(text.byte_count(), 6);
    }

    #[test]
    fn test_str_char_width_unicode() {
        let text = "ab🧑ef";
        assert_eq!(text.char_width(), 6);
    }

    #[test]
    fn test_str_byte_count_unicode() {
        let text = "ab🧑ef";
        assert_eq!(text.byte_count(), 8);
    }

    #[test]
    fn test_list_char_width_empty() {
        let list = StyledContentList::default();
        assert_eq!(list.char_width(), 0);
    }

    #[test]
    fn test_list_char_width_notext() {
        let style1 = ContentStyle::new();
        let style2 = ContentStyle::new().red();
        let style3 = ContentStyle::new().underlined().bold();
        let list = StyledContentList::new(vec![
            style1.apply(Cow::from("")),
            style2.apply(Cow::from("")),
            style3.apply(Cow::from("")),
        ]);
        assert_eq!(list.char_width(), 0);
    }

    #[test]
    fn test_list_char_width() {
        let style1 = ContentStyle::new();
        let style2 = ContentStyle::new().red();
        let style3 = ContentStyle::new().underlined().bold();
        let list = StyledContentList::new(vec![
            style1.apply(Cow::from("hello")),
            style2.apply(Cow::from("")),
            style3.apply(Cow::from("world")),
        ]);
        assert_eq!(list.char_width(), 10);
    }

    #[test]
    fn test_list_split_off_no_split_if_width_less() {
        let style1 = ContentStyle::new();
        let style2 = ContentStyle::new().red().bold();
        let mut list = StyledContentList::new(vec![
            style1.apply(Cow::from("hello")),
            style2.apply(Cow::from("")),
            style2.apply(Cow::from("world")),
        ]);
        assert_eq!(list.char_width(), 10);
        assert_eq!(list.len(), 3);

        let other_list = list.split_off(16);

        assert_eq!(list.char_width(), 10);
        assert_eq!(list.len(), 3);

        assert!(other_list.is_none());
    }

    #[test]
    fn test_list_split_off_no_split_if_width_equal() {
        let style1 = ContentStyle::new();
        let style2 = ContentStyle::new().red().bold();
        let mut list = StyledContentList::new(vec![
            style1.apply(Cow::from("hello")),
            style2.apply(Cow::from("")),
            style2.apply(Cow::from("world")),
        ]);
        assert_eq!(list.char_width(), 10);
        assert_eq!(list.len(), 3);

        let other_list = list.split_off(10);

        assert_eq!(list.char_width(), 10);
        assert_eq!(list.len(), 3);

        assert!(other_list.is_none());
    }

    #[test]
    fn test_list_split_off_split_at_item_border() {
        let style1 = ContentStyle::new();
        let style2 = ContentStyle::new().red().bold();
        let mut list = StyledContentList::new(vec![
            style1.apply(Cow::from("hello")),
            style2.apply(Cow::from("")),
            style2.apply(Cow::from("world")),
            style1.apply(Cow::from("this is a long")),
            style2.apply(Cow::from("line to be split")),
        ]);
        assert_eq!(list.char_width(), 40);
        assert_eq!(list.len(), 5);

        let other_list = list.split_off(10).unwrap();
        assert_eq!(list.char_width(), 10);
        assert_eq!(list.len(), 3);

        assert_eq!(other_list.char_width(), 30);
        assert_eq!(other_list.len(), 2);
    }

    #[test]
    fn test_list_fill_right_empty() {
        let list = StyledContentList::default();
        assert_eq!(list.char_width(), 0);
        let list = list.fill_right('x', 20);
        assert_eq!(list.char_width(), 20);
    }

    #[test]
    fn test_list_fill_right_short() {
        let style = ContentStyle::new().underlined().bold();
        let list = StyledContentList::new(vec![style.apply(Cow::from("hello"))]);
        assert_eq!(list.char_width(), 5);
        let list = list.fill_right('x', 20);
        assert_eq!(list.char_width(), 20);
    }

    #[test]
    fn test_list_fill_right_same() {
        let style = ContentStyle::new().underlined().bold();
        let list = StyledContentList::new(vec![style.apply(Cow::from("hellohellohellohello"))]);
        assert_eq!(list.char_width(), 20);
        let list = list.fill_right('x', 20);
        assert_eq!(list.char_width(), 20);
    }

    #[test]
    fn test_list_fill_right_long() {
        let style = ContentStyle::new().underlined().bold();
        let list =
            StyledContentList::new(vec![style.apply(Cow::from("hellohellohellohellohello"))]);
        assert_eq!(list.char_width(), 25);
        let list = list.fill_right('x', 20);
        assert_eq!(list.char_width(), 25);
    }
}
