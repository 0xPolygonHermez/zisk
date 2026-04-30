use anyhow::Result;
use bytes::{Bytes, BytesMut};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};

struct Inner {
    cursor: Mutex<Cursor<BytesMut>>,
}

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
    pub fn new() -> Self {
        Self::from_bytes_mut(BytesMut::new())
    }

    /// Construct a `ZiskStdin` from an owned `Vec<u8>`.
    /// Zero-copy: the buffer is moved into `Bytes` without reallocation.
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self::from_bytes(Bytes::from(data))
    }

    /// Construct a `ZiskStdin` from `bytes::Bytes`.
    /// Zero-copy if the buffer is uniquely owned (e.g., freshly received data);
    /// otherwise performs a single allocation and copy to obtain a mutable buffer.
    pub fn from_bytes(data: Bytes) -> Self {
        let buf = data.try_into_mut().unwrap_or_else(|shared| BytesMut::from(shared.as_ref()));
        Self::from_bytes_mut(buf)
    }

    fn from_bytes_mut(buf: BytesMut) -> Self {
        Self { inner: Arc::new(Inner { cursor: Mutex::new(Cursor::new(buf)) }) }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to read input file {:?}: {}", path.as_ref(), e))?;
        Ok(Self::from_vec(data))
    }

    /// Create a `ZiskStdin` from a URI string.
    /// - `None` → empty stdin
    /// - `"file://path"` → read from file
    /// - No scheme → treated as a file path
    pub fn from_uri<S: Into<String>>(stdin_uri: Option<S>) -> Result<ZiskStdin> {
        let Some(uri) = stdin_uri else { return Ok(ZiskStdin::new()) };
        let uri = uri.into();
        if let Some(pos) = uri.find("://") {
            let (scheme, path) = uri.split_at(pos);
            let path = &path[3..];
            match scheme {
                "file" => ZiskStdin::from_file(path),
                _ => Err(anyhow::anyhow!("Unknown stdin scheme: {}", scheme)),
            }
        } else {
            ZiskStdin::from_file(uri.as_str())
        }
    }

    pub fn read_data(&self) -> Vec<u8> {
        self.inner.cursor.lock().unwrap().get_ref().to_vec()
    }

    pub fn read_bytes(&self) -> Vec<u8> {
        self.read_raw().expect("Failed to read from stdin buffer")
    }

    pub fn read<T: DeserializeOwned>(&self) -> Result<T> {
        let data =
            self.read_raw().map_err(|e| anyhow::anyhow!("Failed to read from stdin: {}", e))?;
        bincode::deserialize(&data).map_err(|e| anyhow::anyhow!("Failed to deserialize: {}", e))
    }

    pub fn write<T: Serialize>(&self, data: &T) {
        let bytes = bincode::serialize(data).expect("Failed to serialize");
        self.write_slice(&bytes);
    }

    pub fn write_slice(&self, data: &[u8]) {
        let data_len = data.len();
        let total_len = 8 + data_len;
        let padding = (8 - (total_len % 8)) % 8;
        let len_bytes = data_len.to_le_bytes();

        let mut cursor = self.inner.cursor.lock().unwrap();
        cursor.get_mut().extend_from_slice(&len_bytes);
        cursor.get_mut().extend_from_slice(data);
        if padding > 0 {
            cursor.get_mut().extend_from_slice(&vec![0u8; padding]);
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.inner.cursor.lock().unwrap().get_ref().as_ref())?;
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

    pub fn clear(&self) {
        let mut cursor = self.inner.cursor.lock().unwrap();
        cursor.set_position(0);
        cursor.get_mut().clear();
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
