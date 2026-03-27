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
    ///
    /// # Errors
    /// Returns an error if the path contains invalid UTF-8.
    pub fn file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("path contains invalid UTF-8: {:?}", path))?;
        Ok(Self(StreamSource::from_file(path_str)?))
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

        let is_valid = uri.starts_with("quic://") || (cfg!(unix) && uri.starts_with("unix://"));

        if !is_valid {
            #[cfg(unix)]
            anyhow::bail!("stream() requires 'quic://' or 'unix://' scheme. Got: '{}'", uri);
            #[cfg(not(unix))]
            anyhow::bail!(
                "stream() requires 'quic://' scheme. Got: '{}' (unix:// not supported on this platform)",
                uri
            );
        }

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
