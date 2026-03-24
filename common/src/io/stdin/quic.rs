//! QUIC-backed implementations of [`ZiskIORead`] and [`ZiskIOWrite`].
//!
//! [`ZiskQuicStdinWriter`] and [`ZiskQuicStdinReader`] are independent types:
//!
//! - **[`ZiskQuicStdinWriter`]** — server side: binds a UDP port, accepts one
//!   QUIC client, sends each entry as a separate unidirectional QUIC stream.
//!   Implements [`ZiskIOWrite`].
//! - **[`ZiskQuicStdinReader`]** — client side: connects to the writer's
//!   address, accepts one unidirectional stream per entry.
//!   Implements [`ZiskIORead`].
//!
//! # Message boundaries
//!
//! QUIC unidirectional streams provide natural message boundaries — each
//! [`write_slice`](ZiskIOWrite::write_slice) opens a new stream, writes the
//! payload, and finishes the stream.  Each read accepts exactly one stream,
//! so no length-prefix framing is required.
//!
//! # Blocking behaviour
//!
//! The writer's first [`write_slice`] blocks until a reader connects (QUIC
//! `accept()` completes).  Subsequent writes are non-blocking once the
//! connection is established.  The reader connects lazily on the first read.
//!
//! # Runtime ownership
//!
//! [`ZiskQuicStdinWriter`] delegates directly to [`QuicStreamWriter`], which
//! owns its own runtime internally.  [`ZiskQuicStdinReader`] owns a dedicated
//! runtime so that all Quinn I/O operations on the reader share the same
//! event-loop driver across calls (required because [`QuicStreamReader`] uses
//! `run_async`, which would create a fresh runtime per call otherwise).

use std::net::SocketAddr;
use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use tokio::runtime::Runtime;

use crate::io::{
    QuicStreamReader, QuicStreamWriter, StreamRead, StreamWrite, ZiskIORead, ZiskIOWrite,
};

// ── Writer ────────────────────────────────────────────────────────────────────

/// QUIC-backed stdin writer.
///
/// Binds `bind_addr`, waits for one client connection (blocking on the first
/// write), then sends each entry over a fresh unidirectional QUIC stream.
///
/// Delegates directly to [`QuicStreamWriter`], which owns its own Tokio
/// runtime and is already fully synchronous from the caller's perspective.
pub struct ZiskQuicStdinWriter {
    writer: Mutex<QuicStreamWriter>,
}

impl ZiskQuicStdinWriter {
    /// Create a [`ZiskQuicStdinWriter`] that listens on `bind_addr`.
    pub fn new(bind_addr: SocketAddr) -> Result<Self> {
        Ok(Self { writer: Mutex::new(QuicStreamWriter::new(bind_addr)?) })
    }
}

impl ZiskIOWrite for ZiskQuicStdinWriter {
    fn write<T: Serialize>(&self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serialize_into(&mut tmp, data).expect("ZiskQuicStdinWriter: serialization failed");
        self.write_slice(&tmp);
    }

    fn write_slice(&self, data: &[u8]) {
        self.writer.lock().unwrap().write(data).expect("ZiskQuicStdinWriter: write failed");
    }

    fn write_proof(&self, proof: &[u8]) {
        self.write_slice(proof);
    }
}

// ── Reader ────────────────────────────────────────────────────────────────────

/// QUIC-backed stdin reader.
///
/// Connects to `server_addr` lazily on the first read.  Each call to
/// [`read_bytes`](ZiskIORead::read_bytes) accepts the next unidirectional QUIC
/// stream sent by the writer.
pub struct ZiskQuicStdinReader {
    reader: Mutex<QuicStreamReader>,
    /// Accumulates all received payload bytes for [`read_raw_bytes`].
    raw_buffer: Mutex<Vec<u8>>,
    runtime: Runtime,
}

impl ZiskQuicStdinReader {
    /// Create a [`ZiskQuicStdinReader`] that will connect to `server_addr` on
    /// the first read.
    pub fn new(server_addr: SocketAddr) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("Failed to create tokio runtime for ZiskQuicStdinReader")?;
        Ok(Self {
            reader: Mutex::new(QuicStreamReader::new(server_addr)?),
            raw_buffer: Mutex::new(Vec::new()),
            runtime,
        })
    }

    fn next_entry(&self) -> Result<Vec<u8>> {
        let data = self
            .runtime
            .block_on(async { tokio::task::block_in_place(|| self.reader.lock().unwrap().next()) })?
            .ok_or_else(|| anyhow::anyhow!("ZiskQuicStdinReader: connection closed"))?;
        self.raw_buffer.lock().unwrap().extend_from_slice(&data);
        Ok(data)
    }
}

impl ZiskIORead for ZiskQuicStdinReader {
    fn read_raw_bytes(&self) -> Vec<u8> {
        self.raw_buffer.lock().unwrap().clone()
    }

    fn read_bytes(&self) -> Vec<u8> {
        self.next_entry().expect("ZiskQuicStdinReader: failed to read next entry")
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
            .map_err(|e| anyhow::anyhow!("ZiskQuicStdinReader: read failed: {}", e))?;
        bincode::deserialize(&data)
            .map_err(|e| anyhow::anyhow!("ZiskQuicStdinReader: deserialize failed: {}", e))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Point {
        x: i32,
        y: i32,
    }

    /// Unique port per test to avoid bind conflicts.
    fn unique_addr() -> SocketAddr {
        static PORT: AtomicU16 = AtomicU16::new(19100);
        let port = PORT.fetch_add(1, Ordering::Relaxed);
        format!("127.0.0.1:{}", port).parse().unwrap()
    }

    /// Spawn a writer thread and signal `done` after `write_fn` completes.
    /// The writer stays alive until `done` is set so the reader can drain all
    /// streams before the connection is dropped.
    fn spawn_writer(
        addr: SocketAddr,
        write_fn: impl FnOnce(&ZiskQuicStdinWriter) + Send + 'static,
    ) -> (thread::JoinHandle<()>, Arc<std::sync::atomic::AtomicBool>) {
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done_clone = done.clone();
        let handle = thread::spawn(move || {
            let writer = ZiskQuicStdinWriter::new(addr).unwrap();
            write_fn(&writer);
            // Keep connection alive until the reader has finished reading.
            while !done_clone.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(5));
            }
        });
        (handle, done)
    }

    #[test]
    fn write_slice_then_read_bytes_roundtrip() {
        let addr = unique_addr();
        let (handle, done) = spawn_writer(addr, |w| w.write_slice(b"hello"));
        thread::sleep(Duration::from_millis(50));
        let reader = ZiskQuicStdinReader::new(addr).unwrap();
        assert_eq!(reader.read_bytes(), b"hello");
        done.store(true, Ordering::Release);
        handle.join().unwrap();
    }

    #[test]
    fn write_typed_then_read_typed_roundtrip() {
        let addr = unique_addr();
        let (handle, done) = spawn_writer(addr, |w| w.write(&Point { x: 10, y: -3 }));
        thread::sleep(Duration::from_millis(50));
        let reader = ZiskQuicStdinReader::new(addr).unwrap();
        assert_eq!(reader.read::<Point>().unwrap(), Point { x: 10, y: -3 });
        done.store(true, Ordering::Release);
        handle.join().unwrap();
    }

    #[test]
    fn multiple_writes_then_sequential_reads() {
        let addr = unique_addr();
        let (handle, done) = spawn_writer(addr, |w| {
            w.write_slice(b"first");
            w.write_slice(b"second");
            w.write_slice(b"third");
        });
        thread::sleep(Duration::from_millis(50));
        let reader = ZiskQuicStdinReader::new(addr).unwrap();
        assert_eq!(reader.read_bytes(), b"first");
        assert_eq!(reader.read_bytes(), b"second");
        assert_eq!(reader.read_bytes(), b"third");
        done.store(true, Ordering::Release);
        handle.join().unwrap();
    }

    #[test]
    fn read_raw_bytes_accumulates_all_payloads() {
        let addr = unique_addr();
        let (handle, done) = spawn_writer(addr, |w| w.write_slice(b"abc"));
        thread::sleep(Duration::from_millis(50));
        let reader = ZiskQuicStdinReader::new(addr).unwrap();
        let _ = reader.read_bytes();
        assert_eq!(reader.read_raw_bytes(), b"abc");
        done.store(true, Ordering::Release);
        handle.join().unwrap();
    }
}
