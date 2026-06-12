use crate::error::{CommonError, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};

struct Inner {
    data: Mutex<Vec<u8>>,
    cursor: Mutex<Cursor<Vec<u8>>>,
}

/// The `ZiskStdin` struct provides an abstraction for handling standard input data in a flexible manner.
#[derive(Clone)]
pub struct ZiskStdin {
    inner: Arc<Inner>,
}

impl Default for ZiskStdin {
    fn default() -> Self {
        Self::new()
    }
}

impl ZiskStdin {
    /// Creates a new, empty `ZiskStdin` instance.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                data: Mutex::new(Vec::new()),
                cursor: Mutex::new(Cursor::new(Vec::new())),
            }),
        }
    }

    /// Creates a `ZiskStdin` instance from a vector of bytes.
    pub fn from_vec(data: Vec<u8>) -> Self {
        let cursor = Cursor::new(data.clone());
        Self { inner: Arc::new(Inner { data: Mutex::new(data), cursor: Mutex::new(cursor) }) }
    }

    /// Creates a `ZiskStdin` instance by reading data from a file at the specified path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path.as_ref()).map_err(|e| {
            CommonError::Io(format!("Failed to read input file {:?}: {}", path.as_ref(), e))
        })?;
        Ok(Self::from_vec(data))
    }

    /// Create a `ZiskStdin` from a URI string.
    /// - `None` → empty stdin
    /// - `"file://path"` → read from file
    /// - `"inline://[[1,2],[3]]"` → inline input, a JSON array of u64 arrays
    /// - No scheme → treated as a file path
    pub fn from_uri<S: Into<String>>(stdin_uri: Option<S>) -> Result<ZiskStdin> {
        let Some(uri) = stdin_uri else { return Ok(ZiskStdin::new()) };
        let uri = uri.into();
        if let Some(pos) = uri.find("://") {
            let (scheme, path) = uri.split_at(pos);
            let path = &path[3..];
            match scheme {
                "file" => ZiskStdin::from_file(path),
                "inline" => ZiskStdin::from_inline(path),
                _ => Err(CommonError::UnknownScheme(scheme.to_string())),
            }
        } else {
            ZiskStdin::from_file(uri.as_str())
        }
    }

    /// Create a `ZiskStdin` from an inline JSON array of u64 arrays.
    ///
    /// Each inner array is written as one frame via [`write_slice`](Self::write_slice),
    /// so the buffer is byte-identical to a saved `input.bin`: every frame carries an
    /// 8-byte little-endian length prefix and is padded to an 8-byte boundary.
    ///
    /// Example: `"[[1,2],[3],[4,5,6]]"` produces three frames.
    pub fn from_inline(json: &str) -> Result<ZiskStdin> {
        let frames: Vec<Vec<u64>> = serde_json::from_str(json).map_err(|e| {
            CommonError::Invalid(format!(
                "inline input must be a JSON array of u64 arrays, e.g. [[1,2],[3]]; got: {json}: {e}"
            ))
        })?;
        let stdin = ZiskStdin::new();
        for frame in frames {
            let mut bytes = Vec::with_capacity(frame.len() * 8);
            for word in frame {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            stdin.write_slice(&bytes);
        }
        Ok(stdin)
    }

    /// Read the raw byte data from the `ZiskStdin` buffer.
    pub fn read_data(&self) -> Vec<u8> {
        self.inner.data.lock().unwrap().clone()
    }

    /// Read the next frame of data from the `ZiskStdin` buffer as a vector of bytes.
    pub fn read_bytes(&self) -> Vec<u8> {
        self.read_raw().expect("Failed to read from stdin buffer")
    }

    /// Read the next frame of data from the `ZiskStdin` buffer and deserialize it into a value of type `T`.
    pub fn read<T: DeserializeOwned>(&self) -> Result<T> {
        let data = self
            .read_raw()
            .map_err(|e| CommonError::Io(format!("Failed to read from stdin: {}", e)))?;
        bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map(|(v, _)| v)
            .map_err(|e| CommonError::Deserialization(e.to_string()))
    }

    /// Write a serializable value of type `T` to the `ZiskStdin` buffer as a new frame.
    pub fn write<T: Serialize>(&self, data: &T) {
        let bytes = bincode::serde::encode_to_vec(data, bincode::config::standard())
            .expect("Failed to serialize");
        self.write_slice(&bytes);
    }

    /// Write a raw slice of bytes to the `ZiskStdin` buffer as a new frame, prefixed with its length and padded to an 8-byte boundary.
    pub fn write_slice(&self, data: &[u8]) {
        let data_len = data.len();
        let total_len = 8 + data_len;
        let padding = (8 - (total_len % 8)) % 8;
        let len_bytes = data_len.to_le_bytes();

        let mut buf = self.inner.data.lock().unwrap();
        buf.extend_from_slice(&len_bytes);
        buf.extend_from_slice(data);
        if padding > 0 {
            buf.extend_from_slice(&vec![0u8; padding]);
        }

        let mut cursor = self.inner.cursor.lock().unwrap();
        cursor.get_mut().extend_from_slice(&len_bytes);
        cursor.get_mut().extend_from_slice(data);
        if padding > 0 {
            cursor.get_mut().extend_from_slice(&vec![0u8; padding]);
        }
    }

    /// Save the `ZiskStdin` buffer to a file at the specified path.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CommonError::Io(format!(
                    "failed to create parent directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        std::fs::write(path, self.inner.data.lock().unwrap().as_slice()).map_err(|e| {
            CommonError::Io(format!("failed to write stdin to {}: {e}", path.display()))
        })?;
        Ok(())
    }

    /// Reset the read cursor to the beginning.
    pub fn rewind(&self) {
        self.inner.cursor.lock().unwrap().set_position(0);
    }

    /// Alias for `rewind`.
    pub fn reset(&self) {
        self.rewind();
    }

    /// Clear the `ZiskStdin` buffer and reset the cursor.
    pub fn clear(&self) {
        self.inner.data.lock().unwrap().clear();
        let mut cursor = self.inner.cursor.lock().unwrap();
        *cursor = Cursor::new(Vec::new());
    }

    fn read_raw(&self) -> std::io::Result<Vec<u8>> {
        let mut cursor = self.inner.cursor.lock().unwrap();
        let mut len_bytes = [0u8; 8];
        cursor.read_exact(&mut len_bytes)?;
        let len = usize::from_le_bytes(len_bytes);
        let mut data = vec![0u8; len];
        cursor.read_exact(&mut data)?;
        let total_len = 8 + len;
        let padding = (8 - (total_len % 8)) % 8;
        if padding > 0 {
            let mut pad = vec![0u8; padding];
            cursor.read_exact(&mut pad)?;
        }
        Ok(data)
    }
}
