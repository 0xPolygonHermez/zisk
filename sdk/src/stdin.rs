use zisk_common::{io::ZiskStdin as ZiskStdinInner, ZiskProofWithPublicValues};

use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
/// Standard input for a guest program execution or proof.
#[derive(Clone)]
pub struct ZiskStdin(ZiskStdinInner);

impl ZiskStdin {
    /// Creates a new empty memory-based stdin.
    pub fn new() -> Self {
        Self(ZiskStdinInner::new())
    }

    /// Creates a null stdin (no input).
    pub fn null() -> Self {
        Self(ZiskStdinInner::null())
    }

    /// Creates stdin from raw bytes.
    pub fn memory(data: impl AsRef<[u8]>) -> Self {
        let inner = ZiskStdinInner::new();
        inner.write_slice(data.as_ref());
        Self(inner)
    }

    /// Creates stdin from a serializable data structure.
    pub fn from<T: Serialize>(data: &T) -> Self {
        let inner = ZiskStdinInner::new();
        inner.write(data);
        Self(inner)
    }

    /// Creates stdin from a file path.
    ///
    /// # Errors
    /// Returns an error if the path contains invalid UTF-8.
    pub fn file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("path contains invalid UTF-8: {:?}", path))?;
        Ok(Self(ZiskStdinInner::from_uri(Some(path_str.to_owned()))?))
    }

    /// Streams stdin from a URI.
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
        Ok(Self(ZiskStdinInner::from_uri(Some(uri))?))
    }

    /// Reads and deserializes the next value from the stdin buffer.
    pub fn read<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        self.0.read()
    }

    /// Appends a serialized value to the stdin buffer.
    pub fn write<T: Serialize>(&self, data: &T) {
        self.0.write(data);
    }

    /// Appends raw bytes to the stdin buffer.
    pub fn write_slice(&self, data: &[u8]) {
        self.0.write_slice(data);
    }

    /// Appends a serialized proof with its public values to the stdin buffer.
    ///
    /// Used to pass proofs as inputs to aggregation programs.
    pub fn write_proof(&self, proof: &ZiskProofWithPublicValues) {
        self.0.write_proof(proof);
    }

    /// Saves the stdin buffer contents to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        self.0.save(path.as_ref())
    }

    /// Consumes this wrapper and returns the underlying common `ZiskStdin`.
    ///
    /// Useful when passing stdin to lower-level APIs such as `GuestProgram::run`.
    pub fn into_inner(self) -> ZiskStdinInner {
        self.0
    }
}

impl Default for ZiskStdin {
    fn default() -> Self {
        Self::new()
    }
}
