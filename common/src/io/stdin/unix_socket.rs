//! Unix domain socket-backed implementations of [`ZiskIORead`] and [`ZiskIOWrite`].
//!
//! - **[`ZiskUnixSocketStdinWriter`]** — binds the socket, accepts one client,
//!   writes framed entries directly.  Implements [`ZiskIOWrite`].
//! - **[`ZiskUnixSocketStdinReader`]** — connects lazily on the first read.
//!   Implements [`ZiskIORead`].

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;

use super::framing::{prepare_frame, read_frame};
use crate::io::{
    StreamRead, StreamWrite, UnixSocketStreamReader, UnixSocketStreamWriter, ZiskIORead,
    ZiskIOWrite,
};

/// Unix domain socket-backed StdIn writer.
///
/// Binds `SOCK_SEQPACKET` at the given path, accepts one client connection,
/// and sends each framed entry directly to the socket.
///
/// The first [`write_slice`](ZiskIOWrite::write_slice) call will block until
/// the reader connects.
pub struct ZiskUnixSocketStdinWriter {
    writer: Mutex<UnixSocketStreamWriter>,
}

impl ZiskUnixSocketStdinWriter {
    /// Create a [`ZiskUnixSocketStdinWriter`] bound at `path`.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let mut writer = UnixSocketStreamWriter::new(path)?;
        writer.open()?;
        Ok(Self { writer: Mutex::new(writer) })
    }

    fn _write_slice(&self, data: &[u8]) -> Result<()> {
        let frame = prepare_frame(data);
        let mut writer = self.writer.lock().unwrap();
        writer.wait_for_client()?;
        writer.write(&frame)?;
        Ok(())
    }
}

impl ZiskIOWrite for ZiskUnixSocketStdinWriter {
    fn write<T: Serialize>(&self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serialize_into(&mut tmp, data)
            .expect("ZiskUnixSocketStdinWriter: serialization failed");
        self._write_slice(&tmp).expect("ZiskUnixSocketStdinWriter: write_slice failed");
    }

    fn write_slice(&self, data: &[u8]) {
        self._write_slice(data).expect("ZiskUnixSocketStdinWriter: write_slice failed");
    }

    fn write_proof(&self, proof: &[u8]) {
        self._write_slice(proof).expect("ZiskUnixSocketStdinWriter: write_proof failed");
    }
}

/// Unix domain socket-backed reader.
///
/// Connects lazily to the socket on the first read.  Implements [`ZiskIORead`].
pub struct ZiskUnixSocketStdinReader {
    reader: Mutex<UnixSocketStreamReader>,
    raw_buffer: Mutex<Vec<u8>>,
}

impl ZiskUnixSocketStdinReader {
    /// Create a [`ZiskUnixSocketStdinReader`] that will connect to `path` on first read.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            reader: Mutex::new(UnixSocketStreamReader::new(path)?),
            raw_buffer: Mutex::new(Vec::new()),
        })
    }

    /// Receive the next SEQPACKET, log its raw bytes, and parse the framed entry.
    fn next_entry(&self) -> Result<Vec<u8>> {
        let msg = self.reader.lock().unwrap().next()?.ok_or_else(|| {
            anyhow::anyhow!("ZiskUnixSocketStdinReader: connection closed while reading")
        })?;
        self.raw_buffer.lock().unwrap().extend_from_slice(&msg);
        read_frame(&mut Cursor::new(msg))
            .map_err(|e| anyhow::anyhow!("ZiskUnixSocketStdinReader: parse error: {e}"))
    }
}

impl ZiskIORead for ZiskUnixSocketStdinReader {
    fn read_raw_bytes(&self) -> Vec<u8> {
        self.raw_buffer.lock().unwrap().clone()
    }

    fn read_bytes(&self) -> Vec<u8> {
        self.next_entry().expect("ZiskUnixSocketStdinReader: failed to read next framed entry")
    }

    fn read_slice(&self, slice: &mut [u8]) {
        let data = self.read_bytes();
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
            .next_entry()
            .map_err(|e| anyhow::anyhow!("ZiskUnixSocketStdinReader: read failed: {}", e))?;
        bincode::deserialize(&data)
            .map_err(|e| anyhow::anyhow!("ZiskUnixSocketStdinReader: deserialize failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering as AO};
    use std::thread;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Point {
        x: i32,
        y: i32,
    }

    fn unique_path() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, AO::Relaxed);
        PathBuf::from(format!("/tmp/zisk_io_stream_{}_{}.sock", std::process::id(), id))
    }

    /// Spawn the writer on a background thread; returns the reader and a join handle.
    ///
    /// The writer and reader must be on separate threads because `write_slice`
    /// blocks until the reader connects — mirroring real usage where the two
    /// sides always run concurrently.
    fn spawn_writer(
        path: PathBuf,
        write_fn: impl FnOnce(ZiskUnixSocketStdinWriter) + Send + 'static,
    ) -> (ZiskUnixSocketStdinReader, std::thread::JoinHandle<()>) {
        let writer = ZiskUnixSocketStdinWriter::new(&path).unwrap();
        let reader = ZiskUnixSocketStdinReader::new(&path).unwrap();
        let handle = thread::spawn(move || write_fn(writer));
        (reader, handle)
    }

    #[test]
    fn write_slice_then_read_bytes_roundtrip() {
        let (reader, handle) = spawn_writer(unique_path(), |w| w.write_slice(b"hello"));
        assert_eq!(reader.read_bytes(), b"hello");
        handle.join().unwrap();
    }

    #[test]
    fn write_typed_then_read_typed_roundtrip() {
        let value = Point { x: 42, y: -7 };
        let value2 = Point { x: 42, y: -7 };
        let (reader, handle) = spawn_writer(unique_path(), move |w| w.write(&value));
        assert_eq!(reader.read::<Point>().unwrap(), value2);
        handle.join().unwrap();
    }

    #[test]
    fn multiple_writes_then_sequential_reads() {
        let (reader, handle) = spawn_writer(unique_path(), |w| {
            w.write_slice(b"first");
            w.write_slice(b"second");
            w.write_slice(b"third");
        });
        assert_eq!(reader.read_bytes(), b"first");
        assert_eq!(reader.read_bytes(), b"second");
        assert_eq!(reader.read_bytes(), b"third");
        handle.join().unwrap();
    }

    #[test]
    fn read_raw_bytes_accumulates_all_frames() {
        let (reader, handle) = spawn_writer(unique_path(), |w| w.write_slice(b"abc"));
        let _ = reader.read_bytes();
        let raw = reader.read_raw_bytes();
        assert!(raw.len() >= 11);
        assert!(raw.windows(3).any(|w| w == b"abc"));
        handle.join().unwrap();
    }

    #[test]
    fn read_slice_roundtrip() {
        let (reader, handle) = spawn_writer(unique_path(), |w| w.write_slice(b"slice data"));
        let mut buf = vec![0u8; b"slice data".len()];
        reader.read_slice(&mut buf);
        assert_eq!(buf, b"slice data");
        handle.join().unwrap();
    }
}
