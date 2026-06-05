//! cargo-zisk and cargo-zisk-dev cli entry points.
//! This crate provides the `cargo-zisk` and `cargo-zisk-dev` binaries, which are the main entry points for users of Zisk.

#![warn(missing_docs)] // ratchet up to deny once clean
#![warn(rustdoc::all)] // broken intra-doc links, invalid HTML, bare URLs
#![deny(rustdoc::missing_crate_level_docs)]

pub mod commands;
mod common;
mod ux;
