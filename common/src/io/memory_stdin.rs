use serde::Serialize;
use std::io::{Cursor, Read};

use crate::io::ZiskIO;

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

impl ZiskIO for ZiskMemoryStdin {
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

    fn write<T: Serialize>(&mut self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serialize_into(&mut tmp, data).expect("Failed to serialize data into memory");
        self.data.extend_from_slice(&tmp);
        self.cursor.get_mut().extend_from_slice(&tmp);
    }

    fn write_slice(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
        self.cursor.get_mut().extend_from_slice(data);
    }
}
