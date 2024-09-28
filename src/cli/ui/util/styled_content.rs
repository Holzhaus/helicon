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

    /// The length of the (unstyled) contents of this list.
    pub fn len_unstyled(&self) -> usize {
        self.0
            .iter()
            .map(|styled_content| styled_content.content().len())
            .sum()
    }

    /// Split off the items after `max_width`. Modifies the original list and returns the content
    /// that was split off. Returns `None` if the length of its contents is shorter than
    /// `max_width`.
    pub fn split_off(&mut self, width: usize) -> Option<Self> {
        if self.len_unstyled() <= width {
            return None;
        }

        let mut first_width = 0;
        let first_element_count = self
            .0
            .iter()
            .map(|styled_content| styled_content.content().len())
            .take_while(|len| {
                let next_first_width = first_width + len;
                if next_first_width <= width {
                    first_width = next_first_width;
                    true
                } else {
                    false
                }
            })
            .count();
        let mut second_vec = self.0.split_off(first_element_count);

        debug_assert!(first_width <= width);
        debug_assert!(!second_vec.is_empty());
        let chars_left = width - first_width;
        if chars_left >= 5 && second_vec[0].content().len() > (chars_left + 5) {
            let item = second_vec.remove(0);
            let (lhs, rhs) = item.content().split_at(chars_left);
            self.0.push(item.style().apply(Cow::from(lhs.to_owned())));
            second_vec.insert(0, item.style().apply(Cow::from(rhs.to_owned())));
        }

        Some(Self::new(second_vec))
    }

    /// Append an item consisting of fill [`char`] to this list so that the length of its contents
    /// equals `desired_width`.
    pub fn fill_right(mut self, fill_char: char, desired_width: usize) -> Self {
        if self.len_unstyled() >= desired_width {
            return self;
        }

        let missing_chars = desired_width - self.len_unstyled();
        self.0.push(StyledContent::new(
            ContentStyle::new(),
            Cow::from(
                std::iter::repeat(fill_char)
                    .take(missing_chars)
                    .collect::<String>(),
            ),
        ));
        debug_assert_eq!(self.len_unstyled(), desired_width);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Stylize;

    #[test]
    fn test_list_len_unstyled_empty() {
        let list = StyledContentList::default();
        assert_eq!(list.len_unstyled(), 0);
    }

    #[test]
    fn test_list_len_unstyled_notext() {
        let style1 = ContentStyle::new();
        let style2 = ContentStyle::new().red();
        let style3 = ContentStyle::new().underlined().bold();
        let list = StyledContentList::new(vec![
            style1.apply(Cow::from("")),
            style2.apply(Cow::from("")),
            style3.apply(Cow::from("")),
        ]);
        assert_eq!(list.len_unstyled(), 0);
    }

    #[test]
    fn test_list_len_unstyled() {
        let style1 = ContentStyle::new();
        let style2 = ContentStyle::new().red();
        let style3 = ContentStyle::new().underlined().bold();
        let list = StyledContentList::new(vec![
            style1.apply(Cow::from("hello")),
            style2.apply(Cow::from("")),
            style3.apply(Cow::from("world")),
        ]);
        assert_eq!(list.len_unstyled(), 10);
    }

    #[test]
    fn test_list_fill_right_empty() {
        let list = StyledContentList::default();
        assert_eq!(list.len_unstyled(), 0);
        let list = list.fill_right('x', 20);
        assert_eq!(list.len_unstyled(), 20);
    }

    #[test]
    fn test_list_fill_right_short() {
        let style = ContentStyle::new().underlined().bold();
        let list = StyledContentList::new(vec![style.apply(Cow::from("hello"))]);
        assert_eq!(list.len_unstyled(), 5);
        let list = list.fill_right('x', 20);
        assert_eq!(list.len_unstyled(), 20);
    }

    #[test]
    fn test_list_fill_right_same() {
        let style = ContentStyle::new().underlined().bold();
        let list = StyledContentList::new(vec![style.apply(Cow::from("hellohellohellohello"))]);
        assert_eq!(list.len_unstyled(), 20);
        let list = list.fill_right('x', 20);
        assert_eq!(list.len_unstyled(), 20);
    }

    #[test]
    fn test_list_fill_right_long() {
        let style = ContentStyle::new().underlined().bold();
        let list =
            StyledContentList::new(vec![style.apply(Cow::from("hellohellohellohellohello"))]);
        assert_eq!(list.len_unstyled(), 25);
        let list = list.fill_right('x', 20);
        assert_eq!(list.len_unstyled(), 25);
    }
}
