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
}

impl StreamWrite for UnixSocketStreamWriter {
    /// Open/initialize the stream for writing
    ///
    /// Creates a listening socket and waits for a client to connect (blocking).
    fn open(&mut self) -> Result<()> {
        // If we already have a connected socket, we're done
        if self.socket.is_some() {
            return Ok(());
        }

        // Create listener if not exists
        if self.listener_fd.is_none() {
            self.create_listener()?;
        }

        // If we don't have a socket yet, either spawn accept thread or wait for it
        if self.socket.is_none() {
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

            // Block waiting for the client connection
            if let Some(rx) = &self.socket_receiver {
                match rx.recv() {
                    Ok(stream) => {
                        self.socket = Some(stream);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to receive client connection: {}", e));
                    }
                }
            }
        }

        Ok(())
    }

    /// Write data to the stream, returns the number of bytes written.
    ///
    /// With SOCK_SEQPACKET, each write() sends exactly one complete message,
    /// providing natural message boundaries.
    ///
    /// Returns an error if no client is connected yet.
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
                    Err(_) => {
                        return Err(UnixSocketError::NoClientConnected.into());
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
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_single_message() {
        let socket_path = "/tmp/test_unix_socket_single.sock";
        let _ = std::fs::remove_file(socket_path); // Clean up if exists

        let socket_path_clone = socket_path.to_string();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&socket_path_clone).unwrap();

            // Retry write until client connects
            loop {
                if let Err(e) = writer.write(b"Hello, World!") {
                    if let Some(UnixSocketError::NoClientConnected) =
                        e.downcast_ref::<UnixSocketError>()
                    {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    panic!("Unexpected error: {}", e);
                }
                break;
            }

            writer.close().unwrap();
        });

        // Give writer time to start listening
        thread::sleep(Duration::from_millis(100));

        // Reader connects and reads message
        let mut reader = UnixSocketStreamReader::new(socket_path).unwrap();
        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, b"Hello, World!");
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_multiple_messages() {
        let socket_path = "/tmp/test_unix_socket_multi.sock";
        let _ = std::fs::remove_file(socket_path);

        let socket_path_clone = socket_path.to_string();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&socket_path_clone).unwrap();

            // Retry until client connects for first message
            loop {
                if let Err(e) = writer.write(b"First") {
                    if let Some(UnixSocketError::NoClientConnected) =
                        e.downcast_ref::<UnixSocketError>()
                    {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    panic!("Unexpected error: {}", e);
                }
                break;
            }

            writer.write(b"Second message").unwrap();
            writer.write(b"Third message with more data!").unwrap();
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader connects and reads messages
        let mut reader = UnixSocketStreamReader::new(socket_path).unwrap();
        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"First");
        let msg2 = reader.next().unwrap().unwrap();
        assert_eq!(msg2, b"Second message");
        let msg3 = reader.next().unwrap().unwrap();
        assert_eq!(msg3, b"Third message with more data!");
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_message_boundaries() {
        let socket_path = "/tmp/test_unix_socket_boundaries.sock";
        let _ = std::fs::remove_file(socket_path);

        let socket_path_clone = socket_path.to_string();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&socket_path_clone).unwrap();

            // Retry until client connects for first message
            loop {
                if let Err(e) = writer.write(b"ABC") {
                    if let Some(UnixSocketError::NoClientConnected) =
                        e.downcast_ref::<UnixSocketError>()
                    {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    panic!("Unexpected error: {}", e);
                }
                break;
            }

            writer.write(b"DEF").unwrap();
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader should receive each message as discrete unit
        let mut reader = UnixSocketStreamReader::new(socket_path).unwrap();
        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"ABC");
        let msg2 = reader.next().unwrap().unwrap();
        assert_eq!(msg2, b"DEF");
        // Should NOT be concatenated like "ABCDEF"
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_large_message() {
        let socket_path = "/tmp/test_unix_socket_large.sock";
        let _ = std::fs::remove_file(socket_path);

        let socket_path_clone = socket_path.to_string();

        // Create a large message (64KB - within SOCK_SEQPACKET limits)
        let large_data: Vec<u8> = (0..64 * 1024).map(|i| (i % 256) as u8).collect();
        let large_data_clone = large_data.clone();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&socket_path_clone).unwrap();

            // Retry until client connects for first message
            loop {
                if let Err(e) = writer.write(&large_data) {
                    if let Some(UnixSocketError::NoClientConnected) =
                        e.downcast_ref::<UnixSocketError>()
                    {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    panic!("Unexpected error: {}", e);
                }
                break;
            }

            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader receives large message
        let mut reader = UnixSocketStreamReader::new(socket_path).unwrap();
        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, large_data_clone);
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_connection_close() {
        let socket_path = "/tmp/test_unix_socket_close.sock";
        let _ = std::fs::remove_file(socket_path);

        let socket_path_clone = socket_path.to_string();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&socket_path_clone).unwrap();

            // Retry until client connects for first message
            loop {
                if let Err(e) = writer.write(b"Message") {
                    if let Some(UnixSocketError::NoClientConnected) =
                        e.downcast_ref::<UnixSocketError>()
                    {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    panic!("Unexpected error: {}", e);
                }
                break;
            }

            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader receives message
        let mut reader = UnixSocketStreamReader::new(socket_path).unwrap();
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
        let socket_path = "/tmp/test_unix_socket_stress.sock";
        let _ = std::fs::remove_file(socket_path);

        let socket_path_clone = socket_path.to_string();

        const NUM_MESSAGES: usize = 1000;

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = UnixSocketStreamWriter::new(&socket_path_clone).unwrap();

            // Wait for client to connect with first message
            loop {
                if let Err(e) = writer.write(b"START") {
                    if let Some(UnixSocketError::NoClientConnected) =
                        e.downcast_ref::<UnixSocketError>()
                    {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    panic!("Unexpected error: {}", e);
                }
                break;
            }

            // Send many messages rapidly
            for i in 0..NUM_MESSAGES {
                let msg = format!("Message {}", i);
                writer.write(msg.as_bytes()).unwrap();
            }

            writer.write(b"END").unwrap();
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader receives all messages
        let mut reader = UnixSocketStreamReader::new(socket_path).unwrap();

        // Read START marker
        let start = reader.next().unwrap().unwrap();
        assert_eq!(start, b"START");

        // Read all messages and verify order
        for i in 0..NUM_MESSAGES {
            let expected = format!("Message {}", i);
            let msg = reader.next().unwrap().unwrap();
            assert_eq!(msg, expected.as_bytes(), "Message {} mismatch", i);
        }

        // Read END marker
        let end = reader.next().unwrap().unwrap();
        assert_eq!(end, b"END");

        reader.close().unwrap();
        writer_thread.join().unwrap();
    }
}
