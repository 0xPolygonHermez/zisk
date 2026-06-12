use zisk_common::io::StreamSource;

use serde::Serialize;
use std::path::Path;

use crate::input_stream::ZiskStream;
use crate::{Result, SdkError};

/// Source of hints for a guest program execution or proof.
///
/// - `Hints(ZiskHints)` — inline hints data (memory or file)
/// - `Stream(ZiskStream)` — hints delivered via a live gRPC stream
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
}

impl ZiskHints {
    /// Creates a new empty memory-based hints source.
    pub fn new() -> Self {
        Self { source: StreamSource::from_vec(Vec::new()) }
    }

    /// Creates hints from raw bytes.
    pub fn memory(data: impl AsRef<[u8]>) -> Self {
        Self { source: StreamSource::from_slice(data.as_ref()) }
    }

    /// Creates hints from a serializable data structure.
    pub fn from<T: Serialize>(data: &T) -> Self {
        Self {
            source: StreamSource::from_vec(
                bincode::serde::encode_to_vec(data, bincode::config::standard())
                    .expect("Failed to serialize hints data"),
            ),
        }
    }

    /// Creates hints from a file path.
    ///
    /// # Errors
    /// Returns an error if the file does not exist or is not accessible.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(SdkError::InvalidConfig(format!(
                "Hints file not found: {}",
                path.display()
            )));
        }
        Ok(Self { source: StreamSource::from_file(path).map_err(SdkError::backend)? })
    }

    /// Creates hints from a URI string.
    ///
    /// # Supported Schemes
    /// - `file://path/to/file`   → File-based stream
    /// - `unix://path/to/socket` → Unix domain socket stream
    /// - `quic://host:port`      → QUIC network stream
    /// - No scheme               → treated as a file path
    ///
    /// # Errors
    /// Returns an error if the URI scheme is unknown or the resource is not accessible.
    pub fn from_uri<S: Into<String>>(uri: S) -> Result<Self> {
        Ok(Self { source: StreamSource::from_uri(uri).map_err(SdkError::backend)? })
    }

    pub(crate) fn into_inner(self) -> StreamSource {
        self.source
    }
}

impl Default for ZiskHints {
    fn default() -> Self {
        Self::new()
    }
}
