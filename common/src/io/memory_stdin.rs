use std::io::{Cursor, Read};

use crate::io::ZiskStdin;

/// A memory-based implementation of ZiskStdin that reads from in-memory data.
pub struct ZiskMemoryStdin {
    data: Vec<u8>,
    cursor: Cursor<Vec<u8>>,
}

impl ZiskMemoryStdin {
    /// Create a new ZiskMemoryStdin from a vector of bytes.
    pub fn new(data: Vec<u8>) -> Self {
        let cursor = Cursor::new(data.clone());
        ZiskMemoryStdin { data, cursor }
    }

    /// Create a new ZiskMemoryStdin from a string (UTF-8 encoded).
    pub fn from_string(data: String) -> Self {
        Self::new(data.into_bytes())
    }

    /// Create a new ZiskMemoryStdin from a slice of bytes.
    pub fn from_slice(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl ZiskStdin for ZiskMemoryStdin {
    fn read(&mut self) -> Vec<u8> {
        // Return all the data
        self.data.clone()
    }

    fn read_slice(&mut self, slice: &mut [u8]) {
        self.cursor.read_exact(slice).expect("Failed to read slice from memory");
    }

    fn read_into(&mut self, buffer: &mut [u8]) {
        self.cursor.read_exact(buffer).expect("Failed to read into buffer from memory");
    }

    fn write_serialized(&mut self, _data: &[u8]) {
        panic!("Write operations are not supported for ZiskMemoryStdin");
    }

    fn write_bytes(&mut self, _data: &[u8]) {
        panic!("Write operations are not supported for ZiskMemoryStdin");
    }
}
