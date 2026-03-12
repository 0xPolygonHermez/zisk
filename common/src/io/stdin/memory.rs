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

impl ZiskMemoryStdin {
    fn read_raw_data(&self) -> std::io::Result<Vec<u8>> {
        let mut cursor = self.cursor.lock().unwrap();

        let mut len_bytes = [0u8; 8];
        cursor.read_exact(&mut len_bytes)?;
        let len = usize::from_le_bytes(len_bytes);

        let mut data = vec![0u8; len];
        cursor.read_exact(&mut data)?;

        let total_len = 8 + len;
        let padding = (8 - (total_len % 8)) % 8;
        if padding > 0 {
            let mut padding_bytes = vec![0u8; padding];
            cursor.read_exact(&mut padding_bytes)?;
        }

        Ok(data)
    }
}

impl ZiskIO for ZiskMemoryStdin {
    fn read_raw_bytes(&self) -> Vec<u8> {
        self.data.lock().unwrap().clone()
    }

    fn read_bytes(&self) -> Vec<u8> {
        self.read_raw_data().expect("Failed to read into buffer from memory")
    }

    fn read_slice(&self, slice: &mut [u8]) {
        let data = self.read_raw_data().expect("Failed to read slice from memory");
        assert_eq!(
            slice.len(),
            data.len(),
            "Slice length mismatch: expected {}, got {}",
            data.len(),
            slice.len()
        );
        slice.copy_from_slice(&data);
    }

    fn read<T: DeserializeOwned>(&self) -> Result<T> {
        let data = self
            .read_raw_data()
            .map_err(|e| anyhow::anyhow!("Failed to read data from memory: {}", e))?;

        bincode::deserialize(&data)
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

    fn write_proof(&self, proof: &[u8]) {
        self.write_slice(proof);
    }

    fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.data.lock().unwrap().as_slice())?;
        Ok(())
    }
}
