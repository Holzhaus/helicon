// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Utilities for working with crossterm's `StyledContent`.

use super::LayoutItem;
use crossterm::style::{ContentStyle, StyledContent};
use std::borrow::Cow;
use std::fmt;

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

impl<'a> From<LayoutItem<'a>> for StyledContentList<'a> {
    fn from(val: LayoutItem<'a>) -> Self {
        val.prefix
            .into_iter()
            .chain(val.content)
            .chain(val.suffix)
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
