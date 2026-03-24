use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;

use crate::io::{ZiskIORead, ZiskIOWrite};

/// A memory-based implementation of ZiskStdin that reads from in-memory data.
pub struct ZiskMemoryStdin {
    cursor: Mutex<Cursor<Vec<u8>>>,
}

impl ZiskMemoryStdin {
    /// Create a new ZiskMemoryStdin from a vector of bytes.
    pub fn new(data: Vec<u8>) -> Self {
        ZiskMemoryStdin { cursor: Mutex::new(Cursor::new(data)) }
    }

    /// Create a new ZiskMemoryStdin from a string (UTF-8 encoded).
    pub fn from_string(data: String) -> Self {
        Self::new(data.into_bytes())
    }

    /// Create a new ZiskMemoryStdin from a slice of bytes.
    pub fn from_slice(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }

    fn read_raw_data(&self) -> std::io::Result<Vec<u8>> {
        let mut cursor = self.cursor.lock().unwrap();
        super::framing::read_frame(&mut *cursor)
    }

    fn append_framed(cursor: &mut Cursor<Vec<u8>>, payload: &[u8]) {
        super::framing::append_frame(cursor.get_mut(), payload);
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.cursor.lock().unwrap().get_ref().as_slice())?;
        Ok(())
    }
}

impl ZiskIORead for ZiskMemoryStdin {
    fn read_raw_bytes(&self) -> Vec<u8> {
        self.cursor.lock().unwrap().get_ref().clone()
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
}

impl ZiskIOWrite for ZiskMemoryStdin {
    fn write<T: Serialize>(&self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serialize_into(&mut tmp, data).expect("Failed to serialize data into memory");

        let mut cursor = self.cursor.lock().unwrap();
        Self::append_framed(&mut cursor, &tmp);
    }

    fn write_slice(&self, data: &[u8]) {
        let mut cursor = self.cursor.lock().unwrap();
        Self::append_framed(&mut cursor, data);
    }

    fn write_proof(&self, proof: &[u8]) {
        self.write_slice(proof);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use std::thread;

    /// Build the expected on-disk framing for a raw payload slice.
    fn framed(payload: &[u8]) -> Vec<u8> {
        let data_len = payload.len();
        let total_len = 8 + data_len;
        let padding = (8 - (total_len % 8)) % 8;

        let mut out = Vec::with_capacity(total_len + padding);
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend_from_slice(payload);
        out.extend_from_slice(&vec![0u8; padding]);
        out
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[test]
    fn new_empty_has_no_bytes() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        assert!(stdin.read_raw_bytes().is_empty());
    }

    #[test]
    fn from_slice_roundtrips_raw_bytes() {
        let data = b"hello world";
        let stdin = ZiskMemoryStdin::from_slice(data);
        assert_eq!(stdin.read_raw_bytes(), data);
    }

    #[test]
    fn from_string_roundtrips_raw_bytes() {
        let s = "zisk".to_string();
        let stdin = ZiskMemoryStdin::from_string(s.clone());
        assert_eq!(stdin.read_raw_bytes(), s.as_bytes());
    }

    #[test]
    fn write_then_read_typed_roundtrip() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        let value = Point { x: 42, y: -7 };
        stdin.write(&value);
        let got: Point = stdin.read().unwrap();
        assert_eq!(got, value);
    }

    #[test]
    fn multiple_writes_then_sequential_reads() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        stdin.write(&1u64);
        stdin.write(&2u64);
        stdin.write(&3u64);

        assert_eq!(stdin.read::<u64>().unwrap(), 1);
        assert_eq!(stdin.read::<u64>().unwrap(), 2);
        assert_eq!(stdin.read::<u64>().unwrap(), 3);
    }

    #[test]
    fn write_slice_then_read_bytes_roundtrip() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        let payload = b"some raw bytes";
        stdin.write_slice(payload);
        assert_eq!(stdin.read_bytes(), payload);
    }

    #[test]
    fn write_slice_then_read_slice_roundtrip() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        let payload = b"slice data";
        stdin.write_slice(payload);

        let mut buf = vec![0u8; payload.len()];
        stdin.read_slice(&mut buf);
        assert_eq!(buf, payload);
    }

    #[test]
    fn multiple_write_slice_sequential_reads() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        stdin.write_slice(b"first");
        stdin.write_slice(b"second");

        assert_eq!(stdin.read_bytes(), b"first");
        assert_eq!(stdin.read_bytes(), b"second");
    }

    #[test]
    fn read_raw_bytes_reflects_writes() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        stdin.write_slice(b"abc");

        let raw = stdin.read_raw_bytes();
        assert_eq!(raw, framed(b"abc"));
    }

    #[test]
    fn read_raw_bytes_is_stable_across_reads() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        stdin.write_slice(b"data");

        let raw_before = stdin.read_raw_bytes();
        let _ = stdin.read_bytes(); // advance cursor
        let raw_after = stdin.read_raw_bytes();

        // raw bytes are unaffected by cursor position
        assert_eq!(raw_before, raw_after);
    }

    /// total_len = 8 + payload_len; padding = (8 - total_len % 8) % 8
    #[test]
    fn padding_zero_when_already_aligned() {
        // payload_len = 8 → total = 16, padding = 0
        let stdin = ZiskMemoryStdin::new(Vec::new());
        let payload = b"12345678";
        stdin.write_slice(payload);

        let raw = stdin.read_raw_bytes();
        assert_eq!(raw.len(), 16); // 8 header + 8 payload, no padding
        assert_eq!(&raw[8..], payload);
    }

    #[test]
    fn padding_one_byte() {
        // payload_len = 15 → total = 23, padding = 1
        let stdin = ZiskMemoryStdin::new(Vec::new());
        let payload = vec![0xABu8; 15];
        stdin.write_slice(&payload);

        let raw = stdin.read_raw_bytes();
        assert_eq!(raw.len(), 24); // 8 + 15 + 1
        assert_eq!(raw[23], 0); // padding byte is zero
        assert_eq!(stdin.read_bytes(), payload);
    }

    #[test]
    fn padding_seven_bytes() {
        // payload_len = 1 → total = 9, padding = 7
        let stdin = ZiskMemoryStdin::new(Vec::new());
        stdin.write_slice(b"X");

        let raw = stdin.read_raw_bytes();
        assert_eq!(raw.len(), 16); // 8 + 1 + 7
        assert_eq!(&raw[9..], &[0u8; 7]);
        assert_eq!(stdin.read_bytes(), b"X");
    }

    #[test]
    fn save_writes_same_bytes_as_read_raw_bytes() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        stdin.write_slice(b"save me");

        let tmp = std::env::temp_dir().join("zisk_test_memory_stdin.bin");
        stdin.save(&tmp).unwrap();

        let on_disk = std::fs::read(&tmp).unwrap();
        std::fs::remove_file(&tmp).unwrap();

        assert_eq!(on_disk, stdin.read_raw_bytes());
    }

    #[test]
    fn write_proof_is_readable_as_slice() {
        let stdin = ZiskMemoryStdin::new(Vec::new());
        let proof = b"proof bytes";
        stdin.write_proof(proof);
        assert_eq!(stdin.read_bytes(), proof);
    }

    /// Writer threads each append one framed entry; main thread reads them all
    /// after join. Verifies no partial frames are observed.
    #[test]
    fn concurrent_writes_produce_complete_frames() {
        const N: usize = 64;
        let stdin = Arc::new(ZiskMemoryStdin::new(Vec::new()));

        let handles: Vec<_> = (0..N as u64)
            .map(|i| {
                let s = Arc::clone(&stdin);
                thread::spawn(move || s.write(&i))
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // Each read must succeed and produce a valid u64 (no panic = no corrupt frame).
        let mut values: Vec<u64> = (0..N).map(|_| stdin.read::<u64>().unwrap()).collect();
        values.sort_unstable();
        assert_eq!(values, (0..N as u64).collect::<Vec<_>>());
    }
}
