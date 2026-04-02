use zisk_common::io::StreamSource;

use serde::Serialize;
use std::path::Path;

/// Hints source for a guest program execution or proof.
pub struct ZiskHints(StreamSource);

impl ZiskHints {
    /// Creates a new empty memory-based hints source.
    pub fn new() -> Self {
        Self(StreamSource::from_vec(Vec::new()))
    }

    /// Creates hints from raw bytes.
    pub fn memory(data: impl AsRef<[u8]>) -> Self {
        Self(StreamSource::from_slice(data.as_ref()))
    }

    /// Creates hints from a serializable data structure.
    pub fn from<T: Serialize>(data: &T) -> Self {
        Self(StreamSource::from_vec(
            bincode::serialize(data).expect("Failed to serialize hints data"),
        ))
    }

    /// Creates hints from a file path.
    ///
    /// # Errors
    /// Returns an error if the file does not exist or is not accessible.
    pub fn file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            anyhow::bail!("Hints file not found: {}", path.display());
        }
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
