//! Parser library for Zapret-Info CSV lists.
//! Supports dumps from <https://github.com/zapret-info/z-i> and its mirrors.
//!
//! Source code: <https://github.com/im-0/zicsv>

#![cfg_attr(feature = "unstable", warn(unreachable_pub))]
#![forbid(unsafe_code)]
#![warn(unused_results)]
#![cfg_attr(feature = "cargo-clippy", warn(empty_line_after_outer_attr))]
#![cfg_attr(feature = "cargo-clippy", warn(filter_map))]
#![cfg_attr(feature = "cargo-clippy", warn(if_not_else))]
#![cfg_attr(feature = "cargo-clippy", warn(mut_mut))]
#![cfg_attr(feature = "cargo-clippy", warn(non_ascii_literal))]
#![cfg_attr(feature = "cargo-clippy", warn(option_map_unwrap_or))]
#![cfg_attr(feature = "cargo-clippy", warn(option_map_unwrap_or_else))]
#![cfg_attr(feature = "cargo-clippy", warn(single_match_else))]
#![cfg_attr(feature = "cargo-clippy", warn(wrong_pub_self_convention))]
#![cfg_attr(feature = "cargo-clippy", warn(use_self))]
#![cfg_attr(feature = "cargo-clippy", warn(used_underscore_binding))]
#![cfg_attr(feature = "cargo-clippy", warn(print_stdout))]
#![cfg_attr(feature = "cargo-clippy", warn(else_if_without_else))]

extern crate chrono;
extern crate csv;
extern crate encoding;

#[macro_use]
extern crate failure;

extern crate ipnet;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[cfg(feature = "serialization")]
extern crate serde;
#[cfg(feature = "serialization")]
#[macro_use]
extern crate serde_derive;

extern crate url;
#[cfg(feature = "serialization")]
extern crate url_serde;

#[cfg(feature = "serialization")]
mod ipnet_serde;

mod reader;
pub use reader::*;

mod types;
pub use types::*;
