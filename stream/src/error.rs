//! Error type for the ZisK stream crate.
//!
//! [`StreamError`] is the typed error returned across `zisk-stream`'s public
//! surface — including the `StreamRead`/`StreamWrite`/`StreamProcessor`/
//! `StreamSink`/`BytesPushSender` trait methods — replacing the previous
//! `anyhow::Error`. Each variant mirrors the message text the crate produced
//! before, so error output is unchanged; the dynamic detail (and any
//! stringified cause) is carried in the payload.

/// Errors produced by the ZisK stream layer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StreamError {
    /// A socket or transport I/O operation failed (bind, connect, listen,
    /// accept, read, send). The payload preserves the original, fully-formatted
    /// message (including any stringified cause).
    #[error("{0}")]
    Io(String),

    /// The transport was in the wrong state or its lifecycle was interrupted
    /// (not initialized, not connected, closed before drain/peer, superseded,
    /// background thread gone, …).
    #[error("{0}")]
    Transport(String),

    /// Transport configuration failed (e.g. building the QUIC client config).
    #[error("{0}")]
    Config(String),

    /// An unrecognized stream source URI scheme.
    #[error("Unknown stream source scheme: {0}")]
    UnknownScheme(String),

    /// A precondition or argument was invalid, or a miscellaneous failure that
    /// does not fit a more specific variant. The payload preserves the message.
    #[error("{0}")]
    Invalid(String),

    /// A raw `std::io::Error` propagated directly (via `?`) from a transport
    /// operation that had no custom message — preserved transparently. Sites
    /// that attach context use [`StreamError::Io`] instead.
    #[error(transparent)]
    Source(#[from] std::io::Error),

    /// A Unix-domain-socket transport error, preserved so callers can match on
    /// the specific [`UnixSocketError`](crate::UnixSocketError) variant (e.g.
    /// `NoClientConnected`, which is a retryable condition).
    #[cfg(unix)]
    #[error(transparent)]
    Unix(#[from] crate::unix_socket::UnixSocketError),
}

impl StreamError {
    /// Wrap any displayable foreign error (e.g. `anyhow::Error`, `CommonError`)
    /// into [`StreamError::Invalid`]. Used by trait implementors in other
    /// crates at boundaries where the source type cannot be converted via
    /// `From` — either because naming it would re-introduce a dependency, or
    /// because it lives *above* this crate and a `From` impl would cycle.
    /// Use as `.map_err(StreamError::other)`.
    pub fn other(err: impl std::fmt::Display) -> Self {
        Self::Invalid(err.to_string())
    }
}

/// Stream-crate result type: [`Result`](core::result::Result) defaulting its
/// error half to [`StreamError`].
pub type Result<T, E = StreamError> = core::result::Result<T, E>;
