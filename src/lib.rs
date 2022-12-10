// Copyright (c) 2022 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Tagging library.

#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::missing_docs_in_private_items)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![deny(warnings)]
#![deny(absolute_paths_not_starting_with_crate)]
#![deny(elided_lifetimes_in_paths)]
#![deny(explicit_outlives_requirements)]
#![deny(keyword_idents)]
#![deny(let_underscore_drop)]
#![deny(macro_use_extern_crate)]
#![deny(meta_variable_misuse)]
#![deny(missing_abi)]
#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(non_ascii_idents)]
#![deny(noop_method_call)]
#![deny(pointer_structural_match)]
#![deny(rust_2021_incompatible_closure_captures)]
#![deny(rust_2021_incompatible_or_patterns)]
#![deny(rust_2021_prefixes_incompatible_syntax)]
#![deny(rust_2021_prelude_collisions)]
#![deny(single_use_lifetimes)]
#![deny(trivial_casts)]
#![deny(trivial_numeric_casts)]
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unstable_features)]
#![deny(unused_extern_crates)]
#![deny(unused_import_braces)]
#![deny(unused_lifetimes)]
#![deny(unused_macro_rules)]
#![deny(unused_qualifications)]
#![deny(unused_results)]
#![deny(unused_tuple_struct_fields)]
#![deny(variant_size_differences)]

pub mod import;
mod lookup;
mod tag;
mod util;
