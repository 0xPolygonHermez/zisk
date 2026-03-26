use zisk_common::io::ZiskStdin as ZiskStdinImpl;

use serde::Serialize;
use std::path::Path;

/// Standard input for a guest program execution or proof.
#[derive(Clone)]
pub struct ZiskStdin {}

impl ZiskStdin {
    /// Creates a new memory-based stdin.
    pub fn new() -> ZiskStdinImpl {
        Self::memory(Vec::new())
    }

    /// Creates a null stdin (no input).
    pub fn null() -> ZiskStdinImpl {
        ZiskStdinImpl::null()
    }

    /// Creates stdin from raw bytes.
    pub fn memory(data: impl AsRef<[u8]>) -> ZiskStdinImpl {
        let stdin = ZiskStdinImpl::new();
        stdin.write_slice(data.as_ref());
        stdin
    }

    /// Creates stdin from a serializable data structure.
    pub fn from<T: Serialize>(data: &T) -> ZiskStdinImpl {
        let stdin = ZiskStdinImpl::new();
        stdin.write(data);
        stdin
    }

    /// Creates stdin from a file path.
    ///
    /// # Errors
    /// Returns an error if the path contains invalid UTF-8.
    pub fn file(path: impl AsRef<Path>) -> anyhow::Result<ZiskStdinImpl> {
        let path = path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("path contains invalid UTF-8: {:?}", path))?;
        ZiskStdinImpl::from_uri(Some(path_str.to_owned()))
    }

    /// Streams stdin from a URI.
    ///
    /// Supported schemes:
    /// - `quic://` — QUIC transport
    /// - `unix://` — Unix domain socket (Unix systems only)
    ///
    /// # Errors
    /// Returns an error if the URI scheme is not supported.
    pub fn stream(uri: impl Into<String>) -> anyhow::Result<ZiskStdinImpl> {
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

        ZiskStdinImpl::from_uri(Some(uri))
    }
}
