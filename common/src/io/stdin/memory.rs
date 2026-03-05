use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Mutex;

use crate::io::ZiskIO;

/// A memory-based implementation of ZiskStdin that reads from in-memory data.
pub struct ZiskMemoryStdin {
    data: Mutex<Vec<u8>>,
    cursor: Mutex<Cursor<Vec<u8>>>,
}

impl ZiskMemoryStdin {
    /// Create a new ZiskMemoryStdin from a vector of bytes.
    pub fn new(data: Vec<u8>) -> Self {
        let cursor = Mutex::new(Cursor::new(data.clone()));
        ZiskMemoryStdin { data: Mutex::new(data), cursor }
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
    fn read_bytes(&self) -> Vec<u8> {
        // Return all the data
        self.data.lock().unwrap().clone()
    }

    fn read_slice(&self, slice: &mut [u8]) {
        let mut cursor = self.cursor.lock().unwrap();
        cursor.read_exact(slice).expect("Failed to read slice from memory");
    }

    fn read_into(&self, buffer: &mut [u8]) {
        let mut cursor = self.cursor.lock().unwrap();
        cursor.read_exact(buffer).expect("Failed to read into buffer from memory");
    }

    fn read<T: DeserializeOwned>(&self) -> Result<T> {
        let mut cursor = self.cursor.lock().unwrap();
        bincode::deserialize_from(&mut *cursor)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize from memory: {}", e))
    }

    fn write<T: Serialize>(&self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serialize_into(&mut tmp, data).expect("Failed to serialize data into memory");
        
        // Calculate padding for 8-byte alignment
        let data_len = tmp.len();
        let total_len = 8 + data_len; // header + data
        let padding = (8 - (total_len % 8)) % 8;

        // Write 8-byte length header (includes padding)
        let len_bytes = data_len.to_le_bytes();

        self.data.lock().unwrap().extend_from_slice(&len_bytes);
        self.data.lock().unwrap().extend_from_slice(&tmp);

        // Add padding
        if padding > 0 {
            self.data.lock().unwrap().extend_from_slice(&vec![0u8; padding]);
        }

        let mut cursor = self.cursor.lock().unwrap();
        cursor.get_mut().extend_from_slice(&len_bytes);
        cursor.get_mut().extend_from_slice(&tmp);
        if padding > 0 {
            cursor.get_mut().extend_from_slice(&vec![0u8; padding]);
        }
    }

    fn write_slice(&self, data: &[u8]) {
        let data_len = data.len();
        let total_len = 8 + data_len;
        let padding = (8 - (total_len % 8)) % 8;

        let len_bytes = data_len.to_le_bytes();

        self.data.lock().unwrap().extend_from_slice(&len_bytes);
        self.data.lock().unwrap().extend_from_slice(data);

        if padding > 0 {
            self.data.lock().unwrap().extend_from_slice(&vec![0u8; padding]);
        }

        let mut cursor = self.cursor.lock().unwrap();
        cursor.get_mut().extend_from_slice(&len_bytes);
        cursor.get_mut().extend_from_slice(data);
        if padding > 0 {
            cursor.get_mut().extend_from_slice(&vec![0u8; padding]);
        }
    }

    fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.data.lock().unwrap().as_slice())?;
        Ok(())
    }
}
