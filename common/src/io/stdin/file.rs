//! A file-based implementation of ZiskStdin.
//! This module provides functionality to read input data from a file.

use anyhow::Result;
use serde::de::DeserializeOwned;
use std::fs::{self, File};
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::io::ZiskIORead;

/// A file-based implementation of ZiskStdin that reads from a file.
pub struct ZiskFileStdin {
    /// Path to the input file.
    path: PathBuf,
    reader: Mutex<BufReader<File>>,
}

impl ZiskFileStdin {
    /// Create a new FileStdin from a file path.
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if !path_buf.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found at {:?}", path_buf.display()),
            ));
        }

        let file = File::open(&path_buf)?;
        Ok(ZiskFileStdin { path: path_buf, reader: Mutex::new(BufReader::new(file)) })
    }

    /// Read the next framed entry from the `BufReader`.
    ///
    /// On any I/O error the reader is seeked back to the position it held
    /// before this call, leaving it in a consistent state for the next attempt.
    fn read_raw_data(&self) -> std::io::Result<Vec<u8>> {
        let mut reader = self.reader.lock().unwrap();
        let start = reader.stream_position()?;

        let result = super::framing::read_frame(&mut *reader);

        if result.is_err() {
            // Best-effort seek back; ignore secondary errors.
            let _ = reader.seek(SeekFrom::Start(start));
        }

        result
    }
}

impl ZiskIORead for ZiskFileStdin {
    fn read_raw_bytes(&self) -> Vec<u8> {
        fs::read(&self.path).expect("Could not read inputs file")
    }

    fn read_bytes(&self) -> Vec<u8> {
        self.read_raw_data().expect("Failed to read into buffer")
    }

    fn read_slice(&self, slice: &mut [u8]) {
        let data = self.read_raw_data().expect("Failed to read slice");
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
            .map_err(|e| anyhow::anyhow!("Failed to read data from file: {}", e))?;

        bincode::deserialize(&data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize from file: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::io::Write;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Point {
        x: i32,
        y: i32,
    }

    /// Write a framed payload into a `Vec<u8>` (mirrors the memory impl).
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

    /// Write bytes into a uniquely-named temp file and return its path.
    fn tmp_file_with(content: &[u8]) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("zisk_file_stdin_{}_{}.bin", std::process::id(), id));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    #[test]
    fn read_raw_bytes_returns_full_file_contents() {
        let content = framed(b"hello");
        let path = tmp_file_with(&content);
        let stdin = ZiskFileStdin::new(&path).unwrap();
        assert_eq!(stdin.read_raw_bytes(), content);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn read_raw_bytes_is_independent_of_cursor() {
        let content = framed(b"data");
        let path = tmp_file_with(&content);
        let stdin = ZiskFileStdin::new(&path).unwrap();

        let _ = stdin.read_bytes(); // advance BufReader cursor
                                    // read_raw_bytes should still return the full file
        assert_eq!(stdin.read_raw_bytes(), content);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn read_bytes_decodes_single_framed_entry() {
        let path = tmp_file_with(&framed(b"payload"));
        let stdin = ZiskFileStdin::new(&path).unwrap();
        assert_eq!(stdin.read_bytes(), b"payload");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn read_bytes_sequential_entries() {
        let mut content = framed(b"first");
        content.extend(framed(b"second"));
        let path = tmp_file_with(&content);
        let stdin = ZiskFileStdin::new(&path).unwrap();
        assert_eq!(stdin.read_bytes(), b"first");
        assert_eq!(stdin.read_bytes(), b"second");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn read_typed_roundtrip() {
        let value = Point { x: 10, y: -3 };
        let serialized = bincode::serialize(&value).unwrap();
        let path = tmp_file_with(&framed(&serialized));
        let stdin = ZiskFileStdin::new(&path).unwrap();
        let got: Point = stdin.read().unwrap();
        assert_eq!(got, value);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn read_slice_fills_buffer() {
        let path = tmp_file_with(&framed(b"slicedata"));
        let stdin = ZiskFileStdin::new(&path).unwrap();
        let mut buf = vec![0u8; 9];
        stdin.read_slice(&mut buf);
        assert_eq!(&buf, b"slicedata");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn cursor_recovers_after_truncated_read() {
        // Write a valid frame followed by a truncated one (missing payload bytes).
        let mut content = framed(b"good");
        // Truncated: length header says 100 bytes but we only write 3.
        content.extend_from_slice(&100u64.to_le_bytes());
        content.extend_from_slice(b"BAD");
        let path = tmp_file_with(&content);

        let stdin = ZiskFileStdin::new(&path).unwrap();

        // First read succeeds.
        assert_eq!(stdin.read_bytes(), b"good");

        // Second read fails (truncated frame).
        assert!(stdin.read_raw_data().is_err());

        // Cursor should have been restored — a retry of the same position
        // still fails, but does NOT advance past the bad frame.
        assert!(stdin.read_raw_data().is_err());

        std::fs::remove_file(&path).unwrap();
    }
}
