// Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Tagging library.

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::missing_docs_in_private_items)]
#![expect(clippy::too_many_lines)]
#![expect(clippy::doc_markdown)]
#![expect(clippy::module_name_repetitions)]
#![warn(absolute_paths_not_starting_with_crate)]
#![warn(elided_lifetimes_in_paths)]
#![warn(explicit_outlives_requirements)]
#![warn(keyword_idents)]
#![warn(let_underscore_drop)]
#![warn(macro_use_extern_crate)]
#![warn(meta_variable_misuse)]
#![warn(missing_abi)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(noop_method_call)]
#![warn(rust_2021_incompatible_closure_captures)]
#![warn(rust_2021_incompatible_or_patterns)]
#![warn(rust_2021_prefixes_incompatible_syntax)]
#![warn(rust_2021_prelude_collisions)]
#![warn(single_use_lifetimes)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(unstable_features)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]
#![warn(unused_lifetimes)]
#![warn(unused_macro_rules)]
#![warn(unused_qualifications)]
#![warn(unused_results)]
#![warn(dead_code)]
#![warn(variant_size_differences)]

mod cli;
mod config;
mod distance;
mod error;
mod musicbrainz;
mod release;
mod tag;
mod taggedfile;
mod taggedfilecollection;
mod track;
mod util;

pub use self::cli::main;
pub use self::config::Config;
pub use self::error::{ErrorType as Error, Result};
pub use self::taggedfile::TaggedFile;
pub use self::taggedfilecollection::TaggedFileCollection;
