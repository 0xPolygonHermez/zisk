use std::io::{Cursor, Read};

use crate::io::stream::StreamRead;

/// A memory-based implementation of StreamSource that reads from in-memory data.
pub struct MemoryStreamReader {
    data: Vec<u8>,
    cursor: Cursor<Vec<u8>>,
}

impl MemoryStreamReader {
    /// Create a new MemoryStreamReader from a vector of bytes.
    pub fn new(data: Vec<u8>) -> Self {
        let cursor = Cursor::new(data.clone());
        MemoryStreamReader { data, cursor }
    }

    /// Create a new MemoryStreamReader from a string (UTF-8 encoded).
    pub fn from_string(data: String) -> Self {
        Self::new(data.into_bytes())
    }

    /// Create a new MemoryStreamReader from a slice of bytes.
    pub fn from_slice(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl StreamRead for MemoryStreamReader {
    fn open(&mut self) -> anyhow::Result<()> {
        self.cursor.set_position(0);
        Ok(())
    }

    fn next(&mut self) -> anyhow::Result<Option<Vec<u8>>> {
        let mut buffer = Vec::new();
        let bytes_read = self.cursor.read_to_end(&mut buffer)?;
        if bytes_read == 0 {
            Ok(None)
        } else {
            Ok(Some(buffer))
        }
    }

    fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.cursor.position() < self.data.len() as u64
    }
}
