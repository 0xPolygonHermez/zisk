//! Client library for the ZisK coordinator gRPC API.
//!
//! [`CoordinatorClient`] connects to a coordinator and submits work; a
//! submitted job is tracked through a [`Job`] handle (watch events, wait for
//! the result, cancel), and streaming stdin/hints go through [`InputSender`].

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

/// Coordinator connection and top-level RPCs.
pub mod client;
/// Persistent stdin/hints input streams to a running job.
pub mod input_sender;
/// Handle for tracking a submitted job.
pub mod job;

pub use client::CoordinatorClient;
pub use input_sender::{InputSender, InputSenderPushAdapter};
pub use job::{Job, WatchHandle};
