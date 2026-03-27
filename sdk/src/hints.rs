use zisk_common::io::StreamSource;

use serde::Serialize;
use std::path::Path;

/// Hints source for a guest program execution or proof.
pub struct ZiskHints(StreamSource);

impl ZiskHints {
    /// Creates a new empty memory-based hints source.
    pub fn new() -> Self {
        Self::memory(Vec::new())
    }

    /// Creates hints from raw bytes.
    pub fn memory(data: impl Into<Vec<u8>>) -> Self {
        Self(StreamSource::from_vec(data.into()))
    }

    /// Creates hints from a serializable data structure.
    pub fn from<T: Serialize>(data: &T) -> Self {
        Self::memory(bincode::serialize(data).expect("Failed to serialize hints data"))
    }

    /// Creates hints from a file path.
    pub fn file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(Self(StreamSource::from_file(path)?))
    }

    /// Streams hints from a URI.
    ///
    /// Supported schemes:
    /// - `quic://` — QUIC transport
    /// - `unix://` — Unix domain socket (Unix systems only)
    ///
    /// # Errors
    /// Returns an error if the URI scheme is not supported.
    pub fn stream(uri: impl Into<String>) -> anyhow::Result<Self> {
        let uri = uri.into();
        crate::validate_stream_uri(&uri)?;
        Ok(Self(StreamSource::from_uri(uri)?))
    }

    pub(crate) fn into_inner(self) -> StreamSource {
        self.0
    }
}

impl Default for ZiskHints {
    fn default() -> Self {
        Self::new()
    }
}
