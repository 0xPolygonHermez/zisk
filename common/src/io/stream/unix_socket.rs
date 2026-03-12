//! A Unix domain socket implementation of StreamReader and StreamWriter.
//! This module provides functionality to read and write data through Unix sockets
//! using SOCK_SEQPACKET for message-oriented communication with built-in boundaries.

use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};

use super::{StreamRead, StreamWrite};

/// Errors specific to Unix socket operations
#[derive(Debug, thiserror::Error)]
pub enum UnixSocketError {
    #[error("No client connected yet")]
    NoClientConnected,

    #[error("Socket not connected")]
    NotConnected,

    #[error("Failed to write to socket: {0}")]
    WriteFailed(#[from] std::io::Error),
}

/// A Unix domain socket implementation of StreamRead using SOCK_SEQPACKET.
pub struct UnixSocketStreamReader {
    /// The path to the Unix socket to connect to.
    path: PathBuf,

    /// The connected socket for reading
    socket: Option<UnixStream>,
}

impl UnixSocketStreamReader {
    /// Create a new UnixSocketStreamReader that connects to the specified socket path.
    ///
    /// This creates a client socket that connects to the writer to read data.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(UnixSocketStreamReader { path: path.as_ref().to_path_buf(), socket: None })
    }

    /// Connect to the Unix socket with SOCK_SEQPACKET type
    #[cfg(unix)]
    fn connect_socket(&mut self) -> Result<()> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        // Create socket with SOCK_SEQPACKET
        #[cfg(target_os = "linux")]
        let sock_fd =
            unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) };

        #[cfg(not(target_os = "linux"))]
        let sock_fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET, 0) };

        if sock_fd < 0 {
            return Err(anyhow::anyhow!(
                "Failed to create socket: {}",
                std::io::Error::last_os_error()
            ));
        }

        // Set CLOEXEC flag on non-Linux systems
        #[cfg(not(target_os = "linux"))]
        {
            let flags = unsafe { libc::fcntl(sock_fd, libc::F_GETFD) };
            if flags >= 0 {
                unsafe { libc::fcntl(sock_fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
            }
        }

        // Connect to the socket path
        let c_path =
            CString::new(self.path.as_os_str().as_bytes()).context("Invalid socket path")?;

        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as _;

        let path_bytes = c_path.as_bytes_with_nul();
        if path_bytes.len() > addr.sun_path.len() {
            unsafe { libc::close(sock_fd) };
            return Err(anyhow::anyhow!("Socket path too long"));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                path_bytes.as_ptr() as *const i8,
                addr.sun_path.as_mut_ptr(),
                path_bytes.len(),
            );
        }

        let addr_len = std::mem::size_of_val(&addr.sun_family) + path_bytes.len();

        // Retry connect on EINTR
        loop {
            let result = unsafe {
                libc::connect(
                    sock_fd,
                    &addr as *const libc::sockaddr_un as *const libc::sockaddr,
                    addr_len as u32,
                )
            };

            if result < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue; // Retry on EINTR
                }
                unsafe { libc::close(sock_fd) };
                return Err(anyhow::anyhow!("Failed to connect to socket: {}", err));
            }

            break;
        }

        // Convert to UnixStream
        let socket = unsafe { UnixStream::from_raw_fd(sock_fd) };
        self.socket = Some(socket);

        Ok(())
    }
}

impl StreamRead for UnixSocketStreamReader {
    /// Open/initialize the stream for reading
    ///
    /// Connects to the Unix socket server.
    fn open(&mut self) -> Result<()> {
        if self.is_active() {
            return Ok(());
        }

        self.connect_socket()?;
        Ok(())
    }

    /// Reads the next message from the Unix socket.
    ///
    /// With SOCK_SEQPACKET, each recv() reads exactly one complete message,
    /// providing natural message boundaries.
    fn next(&mut self) -> Result<Option<Vec<u8>>> {
        self.open()?;

        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("UnixSocketStreamReader: Socket not connected"))?;

        // Buffer for receiving messages (128KB max for SOCK_SEQPACKET)
        let mut buffer = vec![0u8; 128 * 1024];

        // Use raw recv to detect MSG_TRUNC
        use std::os::unix::io::AsRawFd;
        let fd = socket.as_raw_fd();

        loop {
            let n = unsafe {
                libc::recv(
                    fd,
                    buffer.as_mut_ptr() as *mut libc::c_void,
                    buffer.len(),
                    libc::MSG_TRUNC,
                )
            };

            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue; // Retry on EINTR
                }
                if err.kind() == std::io::ErrorKind::ConnectionReset {
                    return Ok(None);
                }
                return Err(anyhow::anyhow!("Failed to read from socket: {}", err));
            }

            if n == 0 {
                // Connection closed
                return Ok(None);
            }

            let n = n as usize;

            // Check if message was truncated
            if n > buffer.len() {
                return Err(anyhow::anyhow!(
                    "Message truncated: received {} bytes, buffer size {} bytes",
                    n,
                    buffer.len()
                ));
            }

            buffer.truncate(n);
            return Ok(Some(buffer));
        }
    }

    /// Close the stream
    fn close(&mut self) -> Result<()> {
        self.socket = None;
        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.socket.is_some()
    }
}

impl Drop for UnixSocketStreamReader {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// A Unix domain socket implementation of StreamWrite using SOCK_SEQPACKET.
pub struct UnixSocketStreamWriter {
    /// The path to the Unix socket.
    path: PathBuf,

    /// The listening socket file descriptor (server mode)
    listener_fd: Option<i32>,

    /// The connected socket for writing
    socket: Option<UnixStream>,

    /// Receiver for the accepted socket from background thread
    socket_receiver: Option<Receiver<UnixStream>>,

    /// Handle to the accept thread
    accept_thread: Option<JoinHandle<()>>,
}

impl UnixSocketStreamWriter {
    /// Create a new UnixSocketStreamWriter that listens on the specified socket path.
    ///
    /// This creates a server socket that waits for incoming reader connections.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(UnixSocketStreamWriter {
            path: path.as_ref().to_path_buf(),
            listener_fd: None,
            socket: None,
            socket_receiver: None,
            accept_thread: None,
        })
    }

    /// Create the Unix socket with SOCK_SEQPACKET type
    #[cfg(unix)]
    fn create_listener(&mut self) -> Result<()> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        // Remove socket file if it exists and is stale
        if self.path.exists() {
            // Try to detect if socket is stale by attempting connection
            let is_stale = std::os::unix::net::UnixStream::connect(&self.path).is_err();

            if is_stale {
                std::fs::remove_file(&self.path).context("Failed to remove stale socket file")?;
            } else {
                return Err(anyhow::anyhow!(
                    "Socket path {} is already in use",
                    self.path.display()
                ));
            }
        }

        // Create socket with SOCK_SEQPACKET for message boundaries
        #[cfg(target_os = "linux")]
        let sock_fd =
            unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) };

        #[cfg(not(target_os = "linux"))]
        let sock_fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET, 0) };

        if sock_fd < 0 {
            return Err(anyhow::anyhow!(
                "Failed to create socket: {}",
                std::io::Error::last_os_error()
            ));
        }

        // Set CLOEXEC flag on non-Linux systems
        #[cfg(not(target_os = "linux"))]
        {
            let flags = unsafe { libc::fcntl(sock_fd, libc::F_GETFD) };
            if flags >= 0 {
                unsafe { libc::fcntl(sock_fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
            }
        }

        // Bind to the socket path
        let c_path =
            CString::new(self.path.as_os_str().as_bytes()).context("Invalid socket path")?;

        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as _;

        let path_bytes = c_path.as_bytes_with_nul();
        if path_bytes.len() > addr.sun_path.len() {
            unsafe { libc::close(sock_fd) };
            return Err(anyhow::anyhow!("Socket path too long"));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                path_bytes.as_ptr() as *const i8,
                addr.sun_path.as_mut_ptr(),
                path_bytes.len(),
            );
        }

        let addr_len = std::mem::size_of_val(&addr.sun_family) + path_bytes.len();

        let bind_result = unsafe {
            libc::bind(
                sock_fd,
                &addr as *const libc::sockaddr_un as *const libc::sockaddr,
                addr_len as u32,
            )
        };

        if bind_result < 0 {
            let err = std::io::Error::last_os_error();
            unsafe { libc::close(sock_fd) };
            return Err(anyhow::anyhow!("Failed to bind socket: {}", err));
        }

        // Listen for connections
        let listen_result = unsafe { libc::listen(sock_fd, 1) };

        if listen_result < 0 {
            let err = std::io::Error::last_os_error();
            unsafe { libc::close(sock_fd) };
            return Err(anyhow::anyhow!("Failed to listen on socket: {}", err));
        }

        self.listener_fd = Some(sock_fd);
        Ok(())
    }

    /// Check if a client is currently connected.
    ///
    /// Returns `true` if a client is connected and ready to receive data.
    pub fn is_client_connected(&mut self) -> bool {
        // Already have a connection
        if self.socket.is_some() {
            return true;
        }

        // Try to receive socket from accept thread (non-blocking)
        if let Some(rx) = &self.socket_receiver {
            if let Ok(stream) = rx.try_recv() {
                self.socket = Some(stream);
                return true;
            }
        }

        false
    }
}

impl StreamWrite for UnixSocketStreamWriter {
    /// Open/initialize the stream for writing
    ///
    /// Creates a listening socket and spawns a background thread to accept connections.
    /// This is non-blocking - the actual client connection happens lazily on first write.
    fn open(&mut self) -> Result<()> {
        // If we already have a connected socket, we're done
        if self.socket.is_some() {
            return Ok(());
        }

        // Create listener if not exists
        if self.listener_fd.is_none() {
            self.create_listener()?;
        }

        // Spawn accept thread if not already running
        if self.accept_thread.is_none() {
            let listener_fd = self.listener_fd.unwrap();
            let (tx, rx) = mpsc::channel();
            self.socket_receiver = Some(rx);

            let handle = thread::spawn(move || {
                // Retry accept on EINTR
                let conn_fd = loop {
                    let fd = unsafe {
                        libc::accept(listener_fd, std::ptr::null_mut(), std::ptr::null_mut())
                    };

                    if fd < 0 {
                        let err = std::io::Error::last_os_error();
                        if err.kind() == std::io::ErrorKind::Interrupted {
                            continue; // Retry on EINTR
                        }
                        eprintln!("Accept failed: {}", err);
                        return;
                    }

                    break fd;
                };

                // Convert to UnixStream
                let stream = unsafe { UnixStream::from_raw_fd(conn_fd) };

                // Send socket through channel
                let _ = tx.send(stream);
            });

            self.accept_thread = Some(handle);
        }

        Ok(())
    }

    /// Write data to the stream, returns the number of bytes written.
    ///
    /// With SOCK_SEQPACKET, each write() sends exactly one complete message,
    /// providing natural message boundaries.
    ///
    /// Returns `NoClientConnected` error if no client has connected yet.
    /// The caller can retry the write until a client connects.
    fn write(&mut self, item: &[u8]) -> Result<usize> {
        self.open()?;

        // Receive socket from channel if we don't have it yet
        if self.socket.is_none() {
            if let Some(rx) = &self.socket_receiver {
                // Non-blocking check for socket from accept thread
                match rx.try_recv() {
                    Ok(stream) => {
                        self.socket = Some(stream);
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        // Accept thread is running but client hasn't connected yet
                        return Err(UnixSocketError::NoClientConnected.into());
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Accept thread died unexpectedly
                        return Err(anyhow::anyhow!("Accept thread terminated unexpectedly"));
                    }
                }
            }
        }

        let socket = self.socket.as_mut().ok_or(UnixSocketError::NotConnected)?;

        socket.write_all(item).map_err(UnixSocketError::WriteFailed)?;
        Ok(item.len())
    }

    /// Flush any buffered data
    fn flush(&mut self) -> Result<()> {
        if let Some(socket) = self.socket.as_mut() {
            socket.flush()?;
        }
        Ok(())
    }

    /// Close the stream
    fn close(&mut self) -> Result<()> {
        self.flush()?;

        // Clear the socket
        self.socket = None;

        if let Some(fd) = self.listener_fd.take() {
            unsafe { libc::close(fd) };
        }

        // Clean up socket file
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }

        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.socket.is_some()
    }
}

impl Drop for UnixSocketStreamWriter {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    /// Serialize all unix socket tests to prevent fd reuse races.
    ///
    /// When tests run in parallel and one panics, its Drop closes the listener fd
    /// while the accept thread may still be blocked on it. Due to Linux fd reuse,
    /// this can cause other tests' accept() calls to operate on the wrong fd (EINVAL).
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    /// Generate a unique socket path per test.
    fn unique_socket_path(test_name: &str) -> String {
        format!("/tmp/test_unix_socket_{}_pid{}.sock", test_name, std::process::id(),)
    }

    /// Helper: writer retries write until a client connects, panicking on unexpected errors.
    fn write_with_retry(writer: &mut UnixSocketStreamWriter, data: &[u8]) {
        loop {
            match writer.write(data) {
                Ok(_) => break,
                Err(e) => {
                    if e.downcast_ref::<UnixSocketError>()
                        .is_some_and(|ue| matches!(ue, UnixSocketError::NoClientConnected))
                    {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    panic!("Unexpected write error: {}", e);
                }
            }
        }
    }

    /// Synchronization state shared between the writer thread and the main (reader) thread.
    struct WriterSync {
        /// Signaled by the writer after open() completes (bound + listening + accept spawned).
        ready: AtomicBool,
        /// Signaled by the reader when it has finished reading. The writer waits for this
        /// before closing, to prevent the socket from being torn down while the reader
        /// still has buffered messages to read.
        reader_done: AtomicBool,
    }

    /// Helper: spawn writer in a thread with proper synchronization.
    ///
    /// Returns (join_handle, sync_state). The caller must:
    /// 1. Wait for `sync.ready` before connecting the reader.
    /// 2. Set `sync.reader_done` after the reader has finished reading.
    fn spawn_writer_thread(
        socket_path: &str,
        write_fn: impl FnOnce(&mut UnixSocketStreamWriter) + Send + 'static,
    ) -> (JoinHandle<()>, Arc<WriterSync>) {
        let sp = socket_path.to_string();
        let sync = Arc::new(WriterSync {
            ready: AtomicBool::new(false),
            reader_done: AtomicBool::new(false),
        });
        let sync_clone = sync.clone();

        let handle = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&sp).unwrap();
            writer.open().unwrap();
            sync_clone.ready.store(true, Ordering::Release);
            write_fn(&mut writer);
            // Wait for reader to finish before closing, to avoid ECONNRESET
            let start = std::time::Instant::now();
            while !sync_clone.reader_done.load(Ordering::Acquire) {
                if start.elapsed() > Duration::from_secs(5) {
                    panic!("Timed out waiting for reader to finish");
                }
                thread::sleep(Duration::from_millis(1));
            }
            writer.close().unwrap();
        });

        (handle, sync)
    }

    /// Wait until the writer signals it has finished open() (bound + listening + accept spawned).
    fn wait_for_writer(sync: &WriterSync) {
        let start = std::time::Instant::now();
        while !sync.ready.load(Ordering::Acquire) {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("Timed out waiting for writer to become ready");
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn test_single_message() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("single");
        let _ = std::fs::remove_file(&socket_path);

        let (writer_thread, sync) = spawn_writer_thread(&socket_path, |writer| {
            write_with_retry(writer, b"Hello, World!");
        });

        wait_for_writer(&sync);

        let mut reader = UnixSocketStreamReader::new(&socket_path).unwrap();
        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, b"Hello, World!");
        reader.close().unwrap();

        sync.reader_done.store(true, Ordering::Release);
        writer_thread.join().unwrap();
    }

    #[test]
    fn test_multiple_messages() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("multi");
        let _ = std::fs::remove_file(&socket_path);

        let (writer_thread, sync) = spawn_writer_thread(&socket_path, |writer| {
            write_with_retry(writer, b"First");
            writer.write(b"Second message").unwrap();
            writer.write(b"Third message with more data!").unwrap();
        });

        wait_for_writer(&sync);

        let mut reader = UnixSocketStreamReader::new(&socket_path).unwrap();
        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"First");
        let msg2 = reader.next().unwrap().unwrap();
        assert_eq!(msg2, b"Second message");
        let msg3 = reader.next().unwrap().unwrap();
        assert_eq!(msg3, b"Third message with more data!");
        reader.close().unwrap();

        sync.reader_done.store(true, Ordering::Release);
        writer_thread.join().unwrap();
    }

    #[test]
    fn test_message_boundaries() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("boundaries");
        let _ = std::fs::remove_file(&socket_path);

        let (writer_thread, sync) = spawn_writer_thread(&socket_path, |writer| {
            write_with_retry(writer, b"ABC");
            writer.write(b"DEF").unwrap();
        });

        wait_for_writer(&sync);

        // Reader should receive each message as discrete unit
        let mut reader = UnixSocketStreamReader::new(&socket_path).unwrap();
        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"ABC");
        let msg2 = reader.next().unwrap().unwrap();
        assert_eq!(msg2, b"DEF");
        // Should NOT be concatenated like "ABCDEF"
        reader.close().unwrap();

        sync.reader_done.store(true, Ordering::Release);
        writer_thread.join().unwrap();
    }

    #[test]
    fn test_large_message() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("large");
        let _ = std::fs::remove_file(&socket_path);

        // Create a large message (64KB - within SOCK_SEQPACKET limits)
        let large_data: Vec<u8> = (0..64 * 1024).map(|i| (i % 256) as u8).collect();
        let large_data_clone = large_data.clone();

        let (writer_thread, sync) = spawn_writer_thread(&socket_path, move |writer| {
            write_with_retry(writer, &large_data);
        });

        wait_for_writer(&sync);

        let mut reader = UnixSocketStreamReader::new(&socket_path).unwrap();
        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, large_data_clone);
        reader.close().unwrap();

        sync.reader_done.store(true, Ordering::Release);
        writer_thread.join().unwrap();
    }

    #[test]
    fn test_connection_close() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("close");
        let _ = std::fs::remove_file(&socket_path);

        let sp = socket_path.clone();

        let writer_ready = Arc::new(AtomicBool::new(false));
        let writer_ready_clone = writer_ready.clone();
        let reader_connected = Arc::new(AtomicBool::new(false));
        let reader_connected_clone = reader_connected.clone();

        // This test intentionally lets the writer close to verify the reader sees EOF,
        // so we don't use spawn_writer_thread (which defers close).
        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&sp).unwrap();
            writer.open().unwrap();
            writer_ready_clone.store(true, Ordering::Release);

            // Wait for the reader to connect before writing + closing
            let start = std::time::Instant::now();
            while !reader_connected_clone.load(Ordering::Acquire) {
                if start.elapsed() > Duration::from_secs(5) {
                    panic!("Timed out waiting for reader to connect");
                }
                thread::sleep(Duration::from_millis(1));
            }

            write_with_retry(&mut writer, b"Message");
            writer.close().unwrap();
        });

        // Wait for writer to be listening
        let start = std::time::Instant::now();
        while !writer_ready.load(Ordering::Acquire) {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("Timed out waiting for writer");
            }
            thread::sleep(Duration::from_millis(1));
        }

        // Connect reader and signal writer
        let mut reader = UnixSocketStreamReader::new(&socket_path).unwrap();
        reader.open().unwrap();
        reader_connected.store(true, Ordering::Release);

        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"Message");

        // After writer closes, next should return None
        let msg2 = reader.next().unwrap();
        assert_eq!(msg2, None);
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_stress_many_messages() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("stress");
        let _ = std::fs::remove_file(&socket_path);

        const NUM_MESSAGES: usize = 1000;

        let (writer_thread, sync) = spawn_writer_thread(&socket_path, |writer| {
            write_with_retry(writer, b"START");

            for i in 0..NUM_MESSAGES {
                let msg = format!("Message {}", i);
                writer.write(msg.as_bytes()).unwrap();
            }

            writer.write(b"END").unwrap();
        });

        wait_for_writer(&sync);

        let mut reader = UnixSocketStreamReader::new(&socket_path).unwrap();

        let start = reader.next().unwrap().unwrap();
        assert_eq!(start, b"START");

        for i in 0..NUM_MESSAGES {
            let expected = format!("Message {}", i);
            let msg = reader.next().unwrap().unwrap();
            assert_eq!(msg, expected.as_bytes(), "Message {} mismatch", i);
        }

        let end = reader.next().unwrap().unwrap();
        assert_eq!(end, b"END");

        reader.close().unwrap();
        sync.reader_done.store(true, Ordering::Release);
        writer_thread.join().unwrap();
    }

    #[test]
    fn test_non_blocking_open() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("nonblocking");
        let _ = std::fs::remove_file(&socket_path);

        let mut writer = UnixSocketStreamWriter::new(&socket_path).unwrap();

        let start = std::time::Instant::now();
        writer.open().unwrap();
        let elapsed = start.elapsed();

        // open() should return almost immediately (definitely under 100ms)
        assert!(
            elapsed.as_millis() < 100,
            "open() took too long: {:?} - should be non-blocking",
            elapsed
        );

        // But we shouldn't have a client connected yet
        assert!(!writer.is_client_connected());

        // Connect a dummy reader to unblock the accept thread before closing,
        // preventing fd reuse races with the detached accept thread.
        let mut dummy = UnixSocketStreamReader::new(&socket_path).unwrap();
        dummy.open().unwrap();

        writer.close().unwrap();
        // Allow accept thread to fully terminate after fd is closed
        thread::sleep(Duration::from_millis(10));
    }

    #[test]
    fn test_is_client_connected() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("is_connected");
        let _ = std::fs::remove_file(&socket_path);

        let sp = socket_path.clone();

        let sync = Arc::new(WriterSync {
            ready: AtomicBool::new(false),
            reader_done: AtomicBool::new(false),
        });
        let sync_clone = sync.clone();

        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&sp).unwrap();
            writer.open().unwrap();
            sync_clone.ready.store(true, Ordering::Release);

            // Initially, no client should be connected
            assert!(!writer.is_client_connected());

            // Wait for client to connect
            let mut connected = false;
            for _ in 0..200 {
                if writer.is_client_connected() {
                    connected = true;
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }

            assert!(connected, "Client should have connected");

            // After connection, should remain true
            assert!(writer.is_client_connected());

            // Write should now succeed immediately
            writer.write(b"Connected!").unwrap();

            // Wait for reader to finish before closing
            let start = std::time::Instant::now();
            while !sync_clone.reader_done.load(Ordering::Acquire) {
                if start.elapsed() > Duration::from_secs(5) {
                    panic!("Timed out waiting for reader to finish");
                }
                thread::sleep(Duration::from_millis(1));
            }
            writer.close().unwrap();
        });

        wait_for_writer(&sync);

        let mut reader = UnixSocketStreamReader::new(&socket_path).unwrap();
        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, b"Connected!");
        reader.close().unwrap();

        sync.reader_done.store(true, Ordering::Release);
        writer_thread.join().unwrap();
    }

    #[test]
    fn test_wait_for_client_with_is_connected() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("wait_client");
        let _ = std::fs::remove_file(&socket_path);

        let sp = socket_path.clone();

        let sync = Arc::new(WriterSync {
            ready: AtomicBool::new(false),
            reader_done: AtomicBool::new(false),
        });
        let sync_clone = sync.clone();

        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&sp).unwrap();
            writer.open().unwrap();
            sync_clone.ready.store(true, Ordering::Release);

            // Use is_client_connected() to wait for client
            while !writer.is_client_connected() {
                thread::sleep(Duration::from_millis(10));
            }

            // Now write will succeed without retries
            writer.write(b"Message 1").unwrap();
            writer.write(b"Message 2").unwrap();
            writer.write(b"Message 3").unwrap();

            // Wait for reader to finish before closing
            let start = std::time::Instant::now();
            while !sync_clone.reader_done.load(Ordering::Acquire) {
                if start.elapsed() > Duration::from_secs(5) {
                    panic!("Timed out waiting for reader to finish");
                }
                thread::sleep(Duration::from_millis(1));
            }
            writer.close().unwrap();
        });

        wait_for_writer(&sync);

        let mut reader = UnixSocketStreamReader::new(&socket_path).unwrap();
        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"Message 1");
        let msg2 = reader.next().unwrap().unwrap();
        assert_eq!(msg2, b"Message 2");
        let msg3 = reader.next().unwrap().unwrap();
        assert_eq!(msg3, b"Message 3");
        reader.close().unwrap();

        sync.reader_done.store(true, Ordering::Release);
        writer_thread.join().unwrap();
    }

    #[test]
    fn test_no_client_connected_error() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let socket_path = unique_socket_path("no_client");
        let _ = std::fs::remove_file(&socket_path);

        let mut writer = UnixSocketStreamWriter::new(&socket_path).unwrap();
        writer.open().unwrap();

        // Try to write without any client connected
        let result = writer.write(b"Data");

        // Should get NoClientConnected error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<UnixSocketError>().is_some(),
            "Expected UnixSocketError::NoClientConnected"
        );

        // Connect a dummy reader to unblock the accept thread before closing,
        // preventing fd reuse races with the detached accept thread.
        let mut dummy = UnixSocketStreamReader::new(&socket_path).unwrap();
        dummy.open().unwrap();

        writer.close().unwrap();
        // Allow accept thread to fully terminate after fd is closed
        thread::sleep(Duration::from_millis(10));
    }
}
