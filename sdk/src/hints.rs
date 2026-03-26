use zisk_common::io::StreamSource;

use serde::Serialize;
use std::path::Path;

/// Hints source for a guest program execution or proof.
#[derive(Clone)]
pub struct ZiskHints {}

impl ZiskHints {
    /// Creates a new memory-based hints source.
    pub fn new() -> StreamSource {
        Self::memory(Vec::new())
    }

    /// Creates stdin from raw bytes.
    // TODO! pub fn memory(data: impl AsRef<[u8]>) -> StreamSource {
    pub fn memory(data: Vec<u8>) -> StreamSource {
        StreamSource::from_vec(data)
    }

    /// Creates stdin from a serializable data structure.
    pub fn from<T: Serialize>(data: &T) -> StreamSource {
        Self::memory(bincode::serialize(data).expect("Failed to serialize hints data"))
    }

    /// Creates stdin from a file path.
    ///
    /// # Errors
    /// Returns an error if the path contains invalid UTF-8.
    pub fn file(path: impl AsRef<Path>) -> anyhow::Result<StreamSource> {
        let path = path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("path contains invalid UTF-8: {:?}", path))?;
        StreamSource::from_file(path_str)
    }

    /// Streams stdin from a URI.
    ///
    /// Supported schemes:
    /// - `quic://` — QUIC transport
    /// - `unix://` — Unix domain socket (Unix systems only)
    ///
    /// # Errors
    /// Returns an error if the URI scheme is not supported.
    pub fn stream(uri: impl Into<String>) -> anyhow::Result<StreamSource> {
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

        StreamSource::from_uri(uri)
    }
}
