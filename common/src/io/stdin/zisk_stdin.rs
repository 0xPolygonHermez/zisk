#[cfg(unix)]
use crate::io::ZiskUnixSocketStdinReader;
use crate::io::{ZiskFileStdin, ZiskMemoryStdin, ZiskNullStdin, ZiskQuicStdinReader};
use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use std::sync::Arc;

/// Trait for reading from a ZiskStdin source.
pub trait ZiskIORead: Send + Sync {
    /// Read and deserialize a value from the buffer.
    fn read<T: DeserializeOwned>(&self) -> Result<T>;

    fn read_slice(&self, slice: &mut [u8]);

    fn read_raw_bytes(&self) -> Vec<u8>;
    fn read_bytes(&self) -> Vec<u8>;
}

/// Trait for writing to a ZiskStdin source.
pub trait ZiskIOWrite: Send + Sync {
    /// Write a serialized value to the buffer.
    fn write<T: Serialize>(&self, data: &T);

    /// Write a slice of bytes to the buffer.
    fn write_slice(&self, data: &[u8]);

    /// Write proof
    fn write_proof(&self, proof: &[u8]);
}

pub enum ZiskIOVariant {
    File(ZiskFileStdin),
    Null(ZiskNullStdin),
    Memory(ZiskMemoryStdin),
    #[cfg(unix)]
    UnixSocket(ZiskUnixSocketStdinReader),
    Quic(ZiskQuicStdinReader),
}

impl ZiskIORead for ZiskIOVariant {
    fn read_raw_bytes(&self) -> Vec<u8> {
        match self {
            ZiskIOVariant::File(f) => f.read_raw_bytes(),
            ZiskIOVariant::Null(n) => n.read_raw_bytes(),
            ZiskIOVariant::Memory(m) => m.read_raw_bytes(),
            ZiskIOVariant::Quic(s) => s.read_raw_bytes(),
            #[cfg(unix)]
            ZiskIOVariant::UnixSocket(s) => s.read_raw_bytes(),
        }
    }

    fn read_bytes(&self) -> Vec<u8> {
        match self {
            ZiskIOVariant::File(f) => f.read_bytes(),
            ZiskIOVariant::Null(n) => n.read_bytes(),
            ZiskIOVariant::Memory(m) => m.read_bytes(),
            ZiskIOVariant::Quic(s) => s.read_bytes(),
            #[cfg(unix)]
            ZiskIOVariant::UnixSocket(s) => s.read_bytes(),
        }
    }

    fn read_slice(&self, slice: &mut [u8]) {
        match self {
            ZiskIOVariant::File(f) => f.read_slice(slice),
            ZiskIOVariant::Null(n) => n.read_slice(slice),
            ZiskIOVariant::Memory(m) => m.read_slice(slice),
            ZiskIOVariant::Quic(s) => s.read_slice(slice),
            #[cfg(unix)]
            ZiskIOVariant::UnixSocket(s) => s.read_slice(slice),
        }
    }

    fn read<T: DeserializeOwned>(&self) -> Result<T> {
        match self {
            ZiskIOVariant::File(f) => f.read(),
            ZiskIOVariant::Null(n) => n.read(),
            ZiskIOVariant::Memory(m) => m.read(),
            ZiskIOVariant::Quic(s) => s.read(),
            #[cfg(unix)]
            ZiskIOVariant::UnixSocket(s) => s.read(),
        }
    }
}

#[derive(Clone)]
pub struct ZiskStdin {
    io: Arc<ZiskIOVariant>,
}

impl ZiskIORead for ZiskStdin {
    fn read_raw_bytes(&self) -> Vec<u8> {
        self.io.read_raw_bytes()
    }

    fn read_bytes(&self) -> Vec<u8> {
        self.io.read_bytes()
    }

    fn read_slice(&self, slice: &mut [u8]) {
        self.io.read_slice(slice)
    }

    fn read<T: DeserializeOwned>(&self) -> Result<T> {
        self.io.read()
    }
}

impl Default for ZiskStdin {
    fn default() -> Self {
        Self::new()
    }
}

impl ZiskStdin {
    /// Create a new memory-backed stdin.
    pub fn new() -> Self {
        Self { io: Arc::new(ZiskIOVariant::Memory(ZiskMemoryStdin::new(Vec::new()))) }
    }

    /// Create a null stdin (no input).
    pub fn null() -> Self {
        Self { io: Arc::new(ZiskIOVariant::Null(ZiskNullStdin)) }
    }

    /// Create a file-backed stdin.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self { io: Arc::new(ZiskIOVariant::File(ZiskFileStdin::new(path)?)) })
    }

    /// Create a memory-backed stdin pre-loaded with `data`.
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self { io: Arc::new(ZiskIOVariant::Memory(ZiskMemoryStdin::new(data))) }
    }

    /// Create a Unix socket-based stdin
    pub fn from_unix_socket<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Ok(Self {
            io: Arc::new(ZiskIOVariant::UnixSocket(ZiskUnixSocketStdinReader::new(path.as_ref())?)),
        })
    }

    /// Create a QUIC-based stdin
    pub fn from_quic(addr: std::net::SocketAddr) -> Result<Self> {
        Ok(Self { io: Arc::new(ZiskIOVariant::Quic(ZiskQuicStdinReader::new(addr)?)) })
    }

    /// Create a ZiskStdin from a URI string.
    ///
    /// - `None` → null stdin
    /// - `"scheme://path"` → parsed based on scheme
    /// - bare path → treated as a file path
    ///
    /// The `unix://` scheme creates a reader (client side).
    pub fn from_uri<S: Into<String>>(stdin_uri: Option<S>) -> Result<ZiskStdin> {
        let Some(uri) = stdin_uri else {
            return Ok(ZiskStdin::null());
        };

        let uri = uri.into();

        // Check if URI contains "://" separator
        if let Some(pos) = uri.find("://") {
            let (scheme, location) = uri.split_at(pos);
            let path = &location[3..]; // Skip "://"

            match scheme {
                "file" => ZiskStdin::from_file(path),
                #[cfg(unix)]
                "unix" => ZiskStdin::from_unix_socket(path),
                "quic" => ZiskStdin::from_quic(path.parse()?),
                // Unknown scheme - could error or fallback
                _ => Err(anyhow::anyhow!("Unknown stdin URI scheme: {}", scheme)),
            }
        } else {
            // No "://" found - fallback as a file path
            ZiskStdin::from_file(uri.as_str())
        }
    }

    /// Read raw bytes
    pub fn read_raw_bytes(&self) -> Vec<u8> {
        ZiskIORead::read_raw_bytes(self)
    }

    /// Read a value from the buffer.
    pub fn read_bytes(&self) -> Vec<u8> {
        ZiskIORead::read_bytes(self)
    }

    /// Read a slice of bytes from the buffer.
    pub fn read_slice(&self, slice: &mut [u8]) {
        ZiskIORead::read_slice(self, slice)
    }

    /// Read and deserialize a value from the buffer.
    pub fn read<T: DeserializeOwned>(&self) -> Result<T> {
        ZiskIORead::read(self)
    }

    /// Write a serialized value to the buffer.
    pub fn write<T: Serialize>(&self, data: &T) {
        match self.io.as_ref() {
            ZiskIOVariant::Memory(m) => m.write(data),
            _ => panic!("write is only supported on memory- or writer-backed ZiskStdin"),
        }
    }

    pub fn write_slice(&self, data: &[u8]) {
        match self.io.as_ref() {
            ZiskIOVariant::Memory(m) => m.write_slice(data),
            _ => panic!("write_slice is only supported on memory- or writer-backed ZiskStdin"),
        }
    }

    pub fn write_proof(&self, proof: &[u8]) {
        match self.io.as_ref() {
            ZiskIOVariant::Memory(m) => m.write_proof(proof),
            _ => panic!("write_proof is only supported on memory- or writer-backed ZiskStdin"),
        }
    }

    /// Save to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        match self.io.as_ref() {
            ZiskIOVariant::Memory(m) => m.save(path),
            _ => Err(anyhow::anyhow!("save() is only supported on memory-backed ZiskStdin")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::io::Write;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Pair(u32, u32);

    #[test]
    fn new_is_memory_backed_and_writable() {
        let stdin = ZiskStdin::new();
        stdin.write(&42u64);
        assert_eq!(stdin.read::<u64>().unwrap(), 42);
    }

    #[test]
    fn from_vec_preloads_data() {
        let value = Pair(1, 2);
        let mem = ZiskMemoryStdin::new(Vec::new());
        mem.write(&value);
        let raw = mem.read_raw_bytes();

        let stdin = ZiskStdin::from_vec(raw);
        assert_eq!(stdin.read::<Pair>().unwrap(), value);
    }

    #[test]
    fn clone_shares_underlying_buffer() {
        let stdin = ZiskStdin::new();
        stdin.write(&99u64);

        let clone = stdin.clone();
        // Both handles read from the same cursor
        assert_eq!(clone.read::<u64>().unwrap(), 99);
    }

    #[test]
    fn null_read_raw_bytes_is_empty() {
        assert!(ZiskStdin::null().read_raw_bytes().is_empty());
    }

    #[test]
    fn null_read_returns_error() {
        assert!(ZiskStdin::null().read::<u64>().is_err());
    }

    #[test]
    #[should_panic(expected = "writer-backed")]
    fn null_write_panics() {
        ZiskStdin::null().write(&1u64);
    }

    #[test]
    #[should_panic(expected = "writer-backed")]
    fn null_write_slice_panics() {
        ZiskStdin::null().write_slice(b"data");
    }

    #[test]
    #[should_panic(expected = "writer-backed")]
    fn null_write_proof_panics() {
        ZiskStdin::null().write_proof(b"proof");
    }

    fn tmp_framed_file(payload: &[u8]) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("zisk_stdin_test_{}_{}.bin", std::process::id(), id));
        let data_len = payload.len();
        let total_len = 8 + data_len;
        let padding = (8 - (total_len % 8)) % 8;
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&data_len.to_le_bytes()).unwrap();
        f.write_all(payload).unwrap();
        f.write_all(&vec![0u8; padding]).unwrap();
        path
    }

    #[test]
    fn from_file_reads_correctly() {
        let path = tmp_framed_file(b"hello");
        let stdin = ZiskStdin::from_file(&path).unwrap();
        assert_eq!(stdin.read_bytes(), b"hello");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    #[should_panic(expected = "writer-backed")]
    fn from_file_write_panics() {
        let path = tmp_framed_file(b"x");
        let stdin = ZiskStdin::from_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        stdin.write(&1u64);
    }

    #[test]
    #[should_panic(expected = "writer-backed")]
    fn from_file_write_slice_panics() {
        let path = tmp_framed_file(b"x");
        let stdin = ZiskStdin::from_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        stdin.write_slice(b"data");
    }

    #[test]
    fn from_uri_none_gives_null() {
        let stdin = ZiskStdin::from_uri::<String>(None).unwrap();
        assert!(stdin.read_raw_bytes().is_empty());
    }

    #[test]
    fn from_uri_file_scheme_reads_file() {
        let path = tmp_framed_file(b"uri-data");
        let uri = format!("file://{}", path.display());
        let stdin = ZiskStdin::from_uri(Some(uri)).unwrap();
        assert_eq!(stdin.read_bytes(), b"uri-data");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn from_uri_bare_path_reads_file() {
        let path = tmp_framed_file(b"bare-path");
        let stdin = ZiskStdin::from_uri(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(stdin.read_bytes(), b"bare-path");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn from_uri_unknown_scheme_returns_error() {
        assert!(ZiskStdin::from_uri(Some("ftp://example.com/file")).is_err());
    }

    #[cfg(unix)]
    mod stream_tests {
        use super::*;
        use crate::io::ZiskUnixSocketStdinWriter;
        use std::sync::atomic::{AtomicU32, Ordering as AO};
        use std::thread;

        fn unique_socket_path() -> std::path::PathBuf {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let id = COUNTER.fetch_add(1, AO::Relaxed);
            std::path::PathBuf::from(format!(
                "/tmp/zisk_stdin_stream_{}_{}.sock",
                std::process::id(),
                id
            ))
        }

        fn spawn_writer(
            path: std::path::PathBuf,
            write_fn: impl FnOnce(ZiskUnixSocketStdinWriter) + Send + 'static,
        ) -> (ZiskStdin, thread::JoinHandle<()>) {
            let writer = ZiskUnixSocketStdinWriter::new(&path).unwrap();
            let reader = ZiskStdin::from_unix_socket(&path).unwrap();
            let handle = thread::spawn(move || write_fn(writer));
            (reader, handle)
        }

        #[test]
        fn stream_writer_and_reader_roundtrip() {
            let (reader, handle) =
                spawn_writer(unique_socket_path(), |w| w.write_slice(b"stream-data"));
            assert_eq!(reader.read_bytes(), b"stream-data");
            handle.join().unwrap();
        }

        #[test]
        fn write_typed_on_stream_writer_works() {
            let (reader, handle) =
                spawn_writer(unique_socket_path(), |w| w.write(&42u64));
            assert_eq!(reader.read::<u64>().unwrap(), 42);
            handle.join().unwrap();
        }

        #[test]
        fn from_uri_unix_scheme_gives_reader() {
            let path = unique_socket_path();
            let writer = ZiskUnixSocketStdinWriter::new(&path).unwrap();
            let uri = format!("unix://{}", path.display());
            let reader = ZiskStdin::from_uri(Some(uri)).unwrap();
            let handle = thread::spawn(move || writer.write_slice(b"uri-stream"));
            assert_eq!(reader.read_bytes(), b"uri-stream");
            handle.join().unwrap();
        }

        #[test]
        #[should_panic(expected = "writer-backed")]
        fn stream_reader_write_panics() {
            let path = unique_socket_path();
            let _writer = ZiskUnixSocketStdinWriter::new(&path).unwrap();
            let reader = ZiskStdin::from_unix_socket(&path).unwrap();
            reader.write_slice(b"bad");
        }
    }
}
