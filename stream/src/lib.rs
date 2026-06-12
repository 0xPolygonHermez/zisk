//! `zisk-stream`: ZisK's stream IO layer — traits and transports.
//!
//! This crate owns the [`StreamRead`]/[`StreamWrite`] traits and every
//! transport that implements them (file, Unix socket, in-memory, channel, and —
//! behind the `quic` feature — QUIC), plus the [`StreamSource`] reader
//! multiplexer and the [`ZiskStreamWriter`] producer.
//!
//! # Features
//!
//! - **default**: traits + `std`-only transports (file, unix-socket, memory,
//!   channel) + `StreamSource`/`ZiskStreamWriter`. No async/networking deps, so
//!   guest-side consumers (`ziskos`) can depend on this crate without pulling in
//!   the prover stack — which would otherwise create a dependency cycle via
//!   `zisk-core` → `ziskos-hints`.
//! - **`quic`**: adds the QUIC transport (pulls in `quinn` + `tokio` + `rustls`).
//!   Enabled by `zisk-common`; left off by `ziskos`.

mod channel;
mod error;
mod file;
mod memory;
mod stream_reader;
mod stream_source;
mod stream_writer;
mod zisk_stream;
mod zisk_stream_writer;

#[cfg(feature = "quic")]
mod quic;

#[cfg(unix)]
mod unix_socket;

// Named (not glob) so the `Result` alias isn't re-exported through
// `zisk_common::io::*`, where it would shadow `std::result::Result`.
pub use channel::ChannelStreamReader;
pub use error::StreamError;
pub use file::{FileStreamReader, FileStreamWriter};
pub use memory::MemoryStreamReader;
pub use stream_reader::*;
pub use stream_source::*;
pub use stream_writer::*;
pub use zisk_stream::*;
pub use zisk_stream_writer::{BytesPushSender, ZiskStreamWriter};

#[cfg(feature = "quic")]
pub use quic::{QuicStreamReader, QuicStreamWriter};

#[cfg(unix)]
pub use unix_socket::{UnixSocketError, UnixSocketStreamReader, UnixSocketStreamWriter};
