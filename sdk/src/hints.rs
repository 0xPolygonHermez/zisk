use zisk_common::io::StreamSource;

use serde::Serialize;
use std::path::Path;

use crate::input_stream::ZiskStream;

/// Source of hints for a guest program execution or proof.
///
/// - `Hints(ZiskHints)` — data-backed hints (memory, file, or stream URI)
/// - `Stream(ZiskStream)` — hints delivered via a live transport (unix, quic)
pub enum HintsSource {
    Hints(Box<ZiskHints>),
    Stream(Box<ZiskStream>),
}

impl From<ZiskHints> for HintsSource {
    fn from(h: ZiskHints) -> Self {
        HintsSource::Hints(Box::new(h))
    }
}

impl From<ZiskStream> for HintsSource {
    fn from(s: ZiskStream) -> Self {
        HintsSource::Stream(Box::new(s))
    }
}

/// Hints source for a guest program execution or proof.
pub struct ZiskHints {
    source: StreamSource,
    /// URI if this hints source was created via [`stream()`](Self::stream).
    uri: Option<String>,
}

impl ZiskHints {
    /// Creates a new empty memory-based hints source.
    pub fn new() -> Self {
        Self { source: StreamSource::from_vec(Vec::new()), uri: None }
    }

    /// Creates hints from raw bytes.
    pub fn memory(data: impl AsRef<[u8]>) -> Self {
        Self { source: StreamSource::from_slice(data.as_ref()), uri: None }
    }

    /// Creates hints from a serializable data structure.
    pub fn from<T: Serialize>(data: &T) -> Self {
        Self {
            source: StreamSource::from_vec(
                bincode::serialize(data).expect("Failed to serialize hints data"),
            ),
            uri: None,
        }
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
        Ok(Self { source: StreamSource::from_file(path)?, uri: None })
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
        let source = StreamSource::from_uri(&uri)?;
        Ok(Self { source, uri: Some(uri) })
    }

    pub(crate) fn into_inner(self) -> StreamSource {
        self.source
    }

    /// Returns the stream URI if this hints source is stream-backed (quic://, unix://).
    /// Returns `None` for file- or memory-backed hints.
    pub(crate) fn stream_uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }
}

impl Default for ZiskHints {
    fn default() -> Self {
        Self::new()
    }
}
