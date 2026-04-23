use crate::io::{
    ChannelStreamReader, MemoryStreamReader, QuicStreamReader, UnixSocketStreamReader,
};

use super::FileStreamReader;

use anyhow::Result;

/// Core trait for stream reading operations
pub trait StreamRead: Send + 'static {
    /// Open/initialize the stream for reading
    fn open(&mut self) -> Result<()>;

    /// Read the next item from the stream
    /// Returns None when the stream is finished
    fn next(&mut self) -> Result<Option<Vec<u8>>>;

    /// Close the stream
    fn close(&mut self) -> Result<()>;

    /// Check if the stream is currently active
    fn is_active(&self) -> bool;
}

pub enum StreamSource {
    File(FileStreamReader),
    UnixSocket(UnixSocketStreamReader),
    Quic(QuicStreamReader),
    Memory(MemoryStreamReader),
    Channel(ChannelStreamReader),
}

impl StreamSource {
    /// Create a file-based stdin
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Ok(StreamSource::File(FileStreamReader::new(path)?))
    }

    /// Create a memory-based stream from an owned vector.
    pub fn from_vec(data: Vec<u8>) -> Self {
        StreamSource::Memory(MemoryStreamReader::new(data))
    }

    /// Create a memory-based stream from borrowed bytes.
    pub fn from_slice(data: &[u8]) -> Self {
        StreamSource::Memory(MemoryStreamReader::from_slice(data))
    }

    /// Create a Unix socket-based stdin
    pub fn from_unix_socket<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Ok(StreamSource::UnixSocket(UnixSocketStreamReader::new(path.as_ref())?))
    }

    /// Create a QUIC-based stdin
    pub fn from_quic(addr: std::net::SocketAddr) -> Result<Self> {
        Ok(StreamSource::Quic(QuicStreamReader::new(addr)?))
    }

    /// Create a channel-backed stream.  Returns the reader and the sender
    /// through which chunks are pushed (send `None` or drop the sender to
    /// signal EOF).
    pub fn channel() -> (Self, std::sync::mpsc::Sender<Option<Vec<u8>>>) {
        let (reader, tx) = ChannelStreamReader::new_pair();
        (StreamSource::Channel(reader), tx)
    }

    /// Create a StreamSource from a URI string
    ///
    /// # URI Formats
    /// - `None` → null stream (no input)
    /// - `"scheme://resource"` → parsed based on scheme
    /// - No scheme → treated as a file path
    ///
    /// # Supported Schemes
    /// - `file://path/to/file`   → File-based stream
    /// - `unix://path/to/socket` → Unix domain socket stream
    /// - `quic://host:port`      → QUIC network stream (e.g., `quic://127.0.0.1:8080`)
    pub fn from_uri<S: Into<String>>(hints_uri: S) -> Result<StreamSource> {
        let uri_str = hints_uri.into();

        // Check if URI contains "://" separator
        if let Some(pos) = uri_str.find("://") {
            let (scheme, location) = uri_str.split_at(pos);
            let path = &location[3..]; // Skip "://"

            match scheme {
                "file" => Self::from_file(path),
                "unix" => Self::from_unix_socket(path),
                "quic" => Self::from_quic(path.parse()?),
                // Unknown scheme - could error or fallback
                _ => Err(anyhow::anyhow!("Unknown stream source scheme: {}", scheme)),
            }
        } else {
            // No "://" found - fallback as a file path
            StreamSource::from_file(uri_str.as_str())
        }
    }
}

impl StreamRead for StreamSource {
    fn open(&mut self) -> Result<()> {
        match self {
            StreamSource::File(s) => s.open(),
            StreamSource::UnixSocket(s) => s.open(),
            StreamSource::Quic(s) => s.open(),
            StreamSource::Memory(s) => s.open(),
            StreamSource::Channel(s) => s.open(),
        }
    }

    fn next(&mut self) -> Result<Option<Vec<u8>>> {
        match self {
            StreamSource::File(s) => s.next(),
            StreamSource::UnixSocket(s) => s.next(),
            StreamSource::Quic(s) => s.next(),
            StreamSource::Memory(s) => s.next(),
            StreamSource::Channel(s) => s.next(),
        }
    }

    fn close(&mut self) -> Result<()> {
        match self {
            StreamSource::File(s) => s.close(),
            StreamSource::UnixSocket(s) => s.close(),
            StreamSource::Quic(s) => s.close(),
            StreamSource::Memory(s) => s.close(),
            StreamSource::Channel(s) => s.close(),
        }
    }

    fn is_active(&self) -> bool {
        match self {
            StreamSource::File(s) => s.is_active(),
            StreamSource::UnixSocket(s) => s.is_active(),
            StreamSource::Quic(s) => s.is_active(),
            StreamSource::Memory(s) => s.is_active(),
            StreamSource::Channel(s) => s.is_active(),
        }
    }
}
