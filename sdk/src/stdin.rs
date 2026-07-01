use zisk_common::io::ZiskStdin as ZiskStdinInner;

use crate::{Result, SdkError};
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

    /// Creates stdin from a byte vector.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let stdin = Self(ZiskStdinInner::new());
        stdin.write_slice(&bytes);
        stdin
    }

    /// Creates stdin from a file path.
    ///
    /// # Errors
    /// Returns an error if the file does not exist, is not accessible, or the path contains
    /// invalid UTF-8.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(ZiskStdinInner::from_file(path).map_err(SdkError::backend)?))
    }

    /// Creates stdin from a URI string.
    ///
    /// - `None` → empty stdin
    /// - `file://path` → read from file
    /// - `inline://[[1,2],[3]]` → inline input, a JSON array of u64 arrays
    /// - No scheme → treated as a file path
    pub fn from_uri<S: Into<String>>(uri: Option<S>) -> Result<Self> {
        Ok(Self(ZiskStdinInner::from_uri(uri).map_err(SdkError::backend)?))
    }

    /// Reads all data from the stdin buffer.
    pub fn read_data(&self) -> Vec<u8> {
        self.0.read_data()
    }

    /// Reads and deserializes the next value from the stdin buffer.
    pub fn read<T: DeserializeOwned>(&self) -> Result<T> {
        self.0.read().map_err(SdkError::backend)
    }

    /// Reads the next `n` bytes from the stdin buffer.
    pub fn read_bytes(&self) -> Vec<u8> {
        self.0.read_bytes()
    }

    /// Appends a serialized value to the stdin buffer.
    pub fn write<T: Serialize>(&self, data: &T) {
        self.0.write(data);
    }

    /// Appends raw bytes to the stdin buffer.
    pub fn write_slice(&self, data: &[u8]) {
        self.0.write_slice(data);
    }

    /// Saves the stdin buffer contents to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        self.0.save(path.as_ref()).map_err(SdkError::backend)
    }

    /// Reset internal read position so the next read starts from the beginning.
    pub fn reset(&self) {
        self.0.reset();
    }

    /// Rewind the read cursor to the beginning.
    pub fn rewind(&self) {
        self.0.rewind();
    }

    /// Clear the entire input buffer.
    pub fn clear(&self) {
        self.0.clear();
    }

    /// Consumes this wrapper and returns the underlying common `ZiskStdin`.
    pub(crate) fn into_inner(self) -> ZiskStdinInner {
        self.0
    }
}

impl Default for ZiskStdin {
    fn default() -> Self {
        Self::new()
    }
}
