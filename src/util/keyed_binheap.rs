// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! A binary heap similar to [`std::collections::BinaryHeap`] that does not require its items to
//! implement the [`Ord`] trait.
//!
//! Instead, a custom key function is used to derive a value that is `Ord` from each item that is
//! added. The function is only called once per `KeyedBinaryHeap::push()` call on the newly added
//! item. Further `push` call will not execute the key function on the values that are already
//! present in the heap.

use std::cmp::Ordering;
use std::collections::binary_heap::{BinaryHeap, IntoIter};
use std::iter::Map;

/// An item that is stored in [`KeyedBinaryHeap`].
pub struct Item<T, K>
where
    K: Eq + Ord,
{
    /// The value.
    value: T,
    /// The key that is used for ordering (derived by the key function).
    key: K,
}

impl<T, K> PartialEq for Item<T, K>
where
    K: Eq + Ord,
{
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T, K> Eq for Item<T, K> where K: Eq + Ord {}

impl<T, K> Ord for Item<T, K>
where
    K: Eq + Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T, K> PartialOrd for Item<T, K>
where
    K: Eq + Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.key.cmp(&other.key))
    }
}

/// A priority queue implemented with a binary heap.
///
/// See [`std::collections::BinaryHeap`] for details.
pub struct KeyedBinaryHeap<T, F, K>
where
    F: Fn(&T) -> K,
    K: Eq + Ord,
{
    /// The underlying binary heap for the `std` library.
    heap: BinaryHeap<Item<T, K>>,
    /// The key function.
    key_fn: F,
}

impl<T, F, K> KeyedBinaryHeap<T, F, K>
where
    F: Fn(&T) -> K,
    K: Eq + Ord,
{
    /// Create a new binary heap with the given capacity and key function.
    pub fn with_capacity(capacity: usize, key_fn: F) -> Self {
        let heap = BinaryHeap::with_capacity(capacity);
        KeyedBinaryHeap { heap, key_fn }
    }

    /// Create a new binary heap with the given key function.
    pub const fn new(key_fn: F) -> Self {
        let heap = BinaryHeap::new();
        KeyedBinaryHeap { heap, key_fn }
    }

    /// Pushes an item onto the binary heap.
    pub fn push(&mut self, value: T) {
        let key = (self.key_fn)(&value);
        self.heap.push(Item { value, key });
    }

    /// Consumes the BinaryHeap and returns a vector in sorted (ascending) order.
    pub fn into_sorted_vec(self) -> Vec<T> {
        self.heap
            .into_sorted_vec()
            .into_iter()
            .map(|item| item.value)
            .collect::<Vec<T>>()
    }
}

impl<T, F, K> IntoIterator for KeyedBinaryHeap<T, F, K>
where
    F: Fn(&T) -> K,
    K: Eq + Ord,
{
    type Item = T;
    type IntoIter = Map<IntoIter<Item<T, K>>, fn(Item<T, K>) -> T>;

    fn into_iter(self) -> Self::IntoIter {
        self.heap.into_iter().map(|item| item.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyed_binheap_string() {
        fn str_len(value: &impl AsRef<str>) -> usize {
            value.as_ref().len()
        }

        let mut heap = KeyedBinaryHeap::new(str_len);
        heap.push("aa");
        heap.push("a");
        heap.push("aaa");
        assert_eq!(heap.into_sorted_vec(), vec!["a", "aa", "aaa"]);

        let mut heap = KeyedBinaryHeap::new(str_len);
        heap.push("aa".to_string());
        heap.push("a".to_string());
        heap.push("aaa".to_string());
        assert_eq!(
            heap.into_sorted_vec(),
            vec![String::from("a"), String::from("aa"), String::from("aaa")]
        );
    }

    #[test]
    fn test_keyed_binheap_obj() {
        struct SomeObject {
            some_value: u32,
        }

        let obj1 = SomeObject { some_value: 456 };
        let obj2 = SomeObject { some_value: 123 };
        let obj3 = SomeObject { some_value: 0 };

        let mut heap = KeyedBinaryHeap::new(|x: &SomeObject| x.some_value);
        heap.push(obj1);
        heap.push(obj2);
        heap.push(obj3);
        assert_eq!(
            heap.into_sorted_vec()
                .into_iter()
                .map(|obj| obj.some_value)
                .collect::<Vec<_>>(),
            vec![0, 123, 456]
        );
    }
}
