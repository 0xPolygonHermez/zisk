//! A QUIC-based implementation of StreamReader and StreamWriter.
//! This module provides functionality to read and write data over QUIC connections
//! for both local and network communication.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use quinn::{Connection, Endpoint, ServerConfig};
use tokio::runtime::{Handle, Runtime};

use super::{StreamRead, StreamWrite};

/// Ensure crypto provider is initialized (idempotent)
fn ensure_crypto_provider() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Helper: run a future on the given dedicated runtime, correctly handling the
/// case where we may already be inside another tokio runtime (e.g. `#[tokio::main]`
/// or `spawn_blocking`).
fn block_on_dedicated<F: std::future::Future>(rt: &Runtime, f: F) -> F::Output {
    match Handle::try_current() {
        // Already in a runtime context — use block_in_place to exit it first,
        // then call our dedicated runtime's block_on.
        Ok(_) => tokio::task::block_in_place(|| rt.block_on(f)),
        // Not in any runtime — safe to call block_on directly.
        Err(_) => rt.block_on(f),
    }
}

/// A QUIC implementation of StreamRead that receives data over QUIC streams.
pub struct QuicStreamReader {
    /// The QUIC connection
    connection: Option<Connection>,

    /// Client endpoint
    endpoint: Option<Endpoint>,

    /// Dedicated Tokio runtime that owns the client endpoint's IO driver.
    /// Created lazily in `open()` so `new()` can be called from any context.
    runtime: Option<Runtime>,

    /// Server address to connect to
    server_addr: SocketAddr,
}

impl QuicStreamReader {
    /// Create a new QuicStreamReader that connects to the specified server address.
    pub fn new(server_addr: SocketAddr) -> Result<Self> {
        ensure_crypto_provider();
        Ok(QuicStreamReader { connection: None, endpoint: None, runtime: None, server_addr })
    }

    /// Returns the dedicated runtime, creating it on first use.
    fn ensure_runtime(&mut self) -> Result<&Runtime> {
        if self.runtime.is_none() {
            // Builder::build() only spawns threads — it does not call block_on,
            // so it's safe to call even from within another tokio runtime.
            self.runtime = Some(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .context("Failed to create tokio runtime for QUIC reader")?,
            );
        }
        Ok(self.runtime.as_ref().unwrap())
    }
}

impl StreamRead for QuicStreamReader {
    fn open(&mut self) -> Result<()> {
        if self.is_active() {
            return Ok(());
        }

        let server_addr = self.server_addr;
        let rt = self.ensure_runtime()?;
        let (endpoint, connection) = block_on_dedicated(rt, async move {
            let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;

            let rustls_config = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
                .with_no_client_auth();

            let mut client_config = quinn::ClientConfig::new(Arc::new(
                quinn::crypto::rustls::QuicClientConfig::try_from(rustls_config)
                    .map_err(|e| anyhow::anyhow!("Failed to create QUIC client config: {}", e))?,
            ));

            let mut transport_config = quinn::TransportConfig::default();
            transport_config.max_concurrent_uni_streams(1024u32.into());
            client_config.transport_config(Arc::new(transport_config));

            endpoint.set_default_client_config(client_config);

            let connection = endpoint
                .connect(server_addr, "localhost")?
                .await
                .context("Failed to connect to server")?;

            Ok::<_, anyhow::Error>((endpoint, connection))
        })?;

        self.endpoint = Some(endpoint);
        self.connection = Some(connection);

        Ok(())
    }

    /// Reads the next message from a QUIC unidirectional stream.
    fn next(&mut self) -> Result<Option<Vec<u8>>> {
        self.open()?;

        let connection = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("QuicStreamReader: Connection not established"))?
            .clone();

        let rt = self.ensure_runtime()?;
        block_on_dedicated(rt, async move {
            let mut recv = match connection.accept_uni().await {
                Ok(stream) => stream,
                Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                    return Ok(None);
                }
                Err(quinn::ConnectionError::ConnectionClosed(_)) => {
                    return Ok(None);
                }
                Err(quinn::ConnectionError::TimedOut) => {
                    return Ok(None);
                }
                Err(e) => return Err(anyhow::anyhow!("Failed to accept stream: {}", e)),
            };

            let data =
                recv.read_to_end(10 * 1024 * 1024).await.context("Failed to read from stream")?;

            Ok(Some(data))
        })
    }

    /// Close the stream
    fn close(&mut self) -> Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close(0u32.into(), b"closing");
        }
        if let Some(endpoint) = self.endpoint.take() {
            if let Some(rt) = self.runtime.as_ref() {
                block_on_dedicated(rt, async move {
                    endpoint.wait_idle().await;
                });
            }
        }
        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.connection.is_some()
    }
}

impl Drop for QuicStreamReader {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            conn.close(0u32.into(), b"closing");
        }
        self.endpoint.take();
        if let Some(rt) = self.runtime.take() {
            std::thread::spawn(move || drop(rt));
        }
    }
}

/// A QUIC implementation of StreamWrite that sends data over QUIC streams.
pub struct QuicStreamWriter {
    /// The QUIC connection
    connection: Option<Connection>,

    /// Dedicated Tokio runtime that owns the endpoint's IO driver.
    /// Wrapped in `Option` so `Drop` can take it and shut it down on a
    /// background thread (avoiding panics when dropped from async context).
    runtime: Option<Runtime>,

    /// Server endpoint — bound at construction time so clients can connect immediately.
    endpoint: Endpoint,
}

impl QuicStreamWriter {
    /// Create a new QuicStreamWriter that binds the given address immediately.
    ///
    /// A dedicated Tokio runtime is created to drive the endpoint's IO. The port is
    /// ready for incoming connections before the matching `QuicStreamReader` tries to
    /// connect. The actual `accept()` (blocking wait for the client) is deferred to
    /// `open()`.
    pub fn new(bind_addr: SocketAddr) -> Result<Self> {
        ensure_crypto_provider();
        let server_config = Self::configure_server()?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("Failed to create tokio runtime for QUIC writer")?;
        // Enter the dedicated runtime's context so the endpoint registers its UDP
        // socket with this runtime's IO reactor (keeps IO alive as long as the
        // runtime lives).  `enter()` is non-blocking — safe to call from async.
        let _guard = runtime.enter();
        let endpoint = Endpoint::server(server_config, bind_addr)
            .context("Failed to bind QUIC server endpoint")?;
        drop(_guard);
        Ok(QuicStreamWriter { connection: None, runtime: Some(runtime), endpoint })
    }

    /// Returns the actual local address the endpoint is bound to.
    ///
    /// Useful when the endpoint was created with port `0` — the OS assigns a
    /// free port and this method returns the resolved address.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint.local_addr().map_err(|e| anyhow::anyhow!("failed to get local addr: {e}"))
    }

    /// Configure server with self-signed certificate
    fn configure_server() -> Result<ServerConfig> {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .context("Failed to generate certificate")?;

        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into());
        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert);

        let mut server_config = ServerConfig::with_single_cert(vec![cert_der], key)
            .context("Failed to create server config")?;

        // Configure transport for better performance
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_concurrent_uni_streams(1024u32.into());
        server_config.transport_config(Arc::new(transport_config));

        Ok(server_config)
    }
}

impl StreamWrite for QuicStreamWriter {
    /// Prepare the endpoint for connections.
    ///
    /// The endpoint is already bound (`new()` did that), so this is a no-op.
    /// The actual blocking `accept()` is in `wait_for_connection()`.
    fn open(&mut self) -> Result<()> {
        Ok(())
    }

    /// Block until a client has connected (up to 60 seconds).
    fn wait_for_connection(&mut self) -> Result<()> {
        if self.is_active() {
            return Ok(());
        }

        let endpoint = self.endpoint.clone();

        let rt = self.runtime.as_ref().expect("runtime dropped");
        let connection = block_on_dedicated(rt, async move {
            let accept_fut = async {
                let incoming = endpoint.accept().await.context("Failed to accept connection")?;
                incoming.await.context("Failed to establish connection")
            };
            tokio::time::timeout(std::time::Duration::from_secs(60), accept_fut).await.map_err(
                |_| anyhow::anyhow!("Timed out waiting for QUIC client connection (60s)"),
            )?
        })?;

        self.connection = Some(connection);
        Ok(())
    }

    /// Write data to the stream, returns the number of bytes written.
    ///
    /// Each call to write() opens a new unidirectional stream, writes the data,
    /// and closes the stream, providing natural message boundaries.
    fn write(&mut self, item: &[u8]) -> Result<usize> {
        self.open()?;

        let connection = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("QuicStreamWriter: Connection not established"))?
            .clone();

        let len = item.len();
        let data = item.to_vec();

        let rt = self.runtime.as_ref().expect("runtime dropped");
        block_on_dedicated(rt, async move {
            // Open a new unidirectional stream for this message
            let mut send = connection.open_uni().await.context("Failed to open stream")?;

            // Write all data
            send.write_all(&data).await.context("Failed to write to stream")?;

            // Finish the stream (signals end of message)
            send.finish().context("Failed to finish stream")?;

            Ok(len)
        })
    }

    /// Flush any buffered data
    ///
    /// QUIC handles flushing automatically, so this is a no-op.
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    /// Close the stream
    fn close(&mut self) -> Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close(0u32.into(), b"closing");
        }
        // Close the endpoint and wait for it to go idle, then shut down the
        // runtime — all while still inside the runtime context so the IO
        // reactor is still live for the async close handshake.
        if let Some(rt) = self.runtime.as_ref() {
            let endpoint = self.endpoint.clone();
            block_on_dedicated(rt, async move {
                endpoint.close(0u32.into(), b"closing");
                endpoint.wait_idle().await;
            });
        }
        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.connection.is_some()
    }

    /// Each write opens one unidirectional QUIC stream; the reader caps it at 10 MB.
    fn max_message_size(&self) -> usize {
        4 * 1024 * 1024
    }
}

impl Drop for QuicStreamWriter {
    fn drop(&mut self) {
        // close() handles connection + endpoint shutdown under the runtime context.
        let _ = self.close();
        // Move the (now-idle) runtime to a background thread so we don't panic
        // when dropped from within an async context.
        if let Some(rt) = self.runtime.take() {
            std::thread::spawn(move || drop(rt));
        }
    }
}

/// Certificate verifier that accepts any certificate (for development only!)
///
/// ⚠️ WARNING: This is INSECURE and should NEVER be used in production.
/// It accepts all certificates without validation, making you vulnerable to MITM attacks.
/// For production use, implement proper certificate validation.
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    // Initialize crypto provider once for all tests
    fn init_crypto() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_single_message() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15001".parse().unwrap();

        // Channel to signal when writer has written data
        let (tx, rx) = mpsc::channel();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"Hello, QUIC!").unwrap();
            tx.send(()).unwrap(); // Signal that data is written

            // Wait for reader to finish before closing
            thread::sleep(Duration::from_millis(500));
            writer.close().unwrap();
        });

        // Give writer time to start listening
        thread::sleep(Duration::from_millis(100));

        // Reader connects (this triggers the writer's connection accept)
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        reader.open().unwrap(); // Explicitly connect

        // Wait for writer to have written data
        rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, b"Hello, QUIC!");
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_multiple_messages() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15002".parse().unwrap();

        // Channel to signal when writer has written data
        let (tx, rx) = mpsc::channel();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"First").unwrap();
            writer.write(b"Second message").unwrap();
            writer.write(b"Third message with more data!").unwrap();
            tx.send(()).unwrap(); // Signal that data is written

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader connects
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        reader.open().unwrap(); // Explicitly connect

        // Wait for writer to have written data
        rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"First");
        let msg2 = reader.next().unwrap().unwrap();
        assert_eq!(msg2, b"Second message");
        let msg3 = reader.next().unwrap().unwrap();
        assert_eq!(msg3, b"Third message with more data!");
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_message_boundaries() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15003".parse().unwrap();

        // Channel to signal when writer has written data
        let (tx, rx) = mpsc::channel();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"ABC").unwrap();
            writer.write(b"DEF").unwrap();
            tx.send(()).unwrap(); // Signal that data is written

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader connects
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        reader.open().unwrap(); // Explicitly connect

        // Wait for writer to have written data
        rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"ABC");
        let msg2 = reader.next().unwrap().unwrap();
        assert_eq!(msg2, b"DEF");
        // Should NOT be concatenated like "ABCDEF"
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_large_message() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15004".parse().unwrap();

        // Create a large message (1MB - QUIC can handle this)
        let large_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
        let large_data_clone = large_data.clone();

        // Channel to signal when writer has written data
        let (tx, rx) = mpsc::channel();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(&large_data).unwrap();
            tx.send(()).unwrap(); // Signal that data is written

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader connects
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        reader.open().unwrap(); // Explicitly connect

        // Wait for writer to have written data
        rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, large_data_clone);
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_connection_close() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15005".parse().unwrap();

        // Channel to signal when writer has written data
        let (tx, rx) = mpsc::channel();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"Message").unwrap();
            tx.send(()).unwrap(); // Signal that data is written

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader connects
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        reader.open().unwrap(); // Explicitly connect

        // Wait for writer to have written data
        rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"Message");
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_multiple_concurrent_messages() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15006".parse().unwrap();

        // Channel to signal when writer has written data
        let (tx, rx) = mpsc::channel();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            for i in 0..10 {
                writer.write(format!("Message {}", i).as_bytes()).unwrap();
            }
            tx.send(()).unwrap(); // Signal that data is written

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader connects
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        reader.open().unwrap(); // Explicitly connect

        // Wait for writer to have written data
        rx.recv_timeout(Duration::from_secs(5)).unwrap();

        for i in 0..10 {
            let msg = reader.next().unwrap().unwrap();
            assert_eq!(msg, format!("Message {}", i).as_bytes());
        }
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_writer_closes_early() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15007".parse().unwrap();

        // Channel to signal when writer has written data
        let (tx, rx) = mpsc::channel();

        // Spawn writer (server) thread that closes after writing one message
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"First").unwrap();
            tx.send(()).unwrap(); // Signal that data is written

            // Writer closes after a short delay
            thread::sleep(Duration::from_millis(100));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(100));

        // Reader connects
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        reader.open().unwrap(); // Explicitly connect

        // Wait for writer to have written data
        rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"First");

        reader.close().unwrap();
        writer_thread.join().unwrap();
    }

    /// Mimics the sha-hasher pattern: writer created from async context,
    /// reader connecting from spawn_blocking.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_writer_from_async_reader_from_blocking() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15008".parse().unwrap();

        // Writer created in the async context (like ZiskStdin::from_stream in #[tokio::main])
        let writer = QuicStreamWriter::new(server_addr).unwrap();

        let writer = std::sync::Arc::new(std::sync::Mutex::new(writer));
        let writer2 = writer.clone();

        // Writer writes from spawn_blocking first (like flush()).
        // write() calls open() internally, which blocks on accept().
        let write_handle = tokio::task::spawn_blocking(move || {
            let mut w = writer2.lock().unwrap();
            eprintln!("[test] writer.write starting...");
            w.write(b"from async writer").unwrap();
            eprintln!("[test] writer.write done");
            thread::sleep(Duration::from_millis(500));
            w.close().unwrap();
        });

        // Give writer time to start listening for accept
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Reader connects from a plain thread
        let reader_thread = thread::spawn(move || {
            eprintln!("[test] reader.open starting...");
            let mut reader = QuicStreamReader::new(server_addr).unwrap();
            reader.open().unwrap();
            eprintln!("[test] reader connected");

            let msg = reader.next().unwrap().unwrap();
            assert_eq!(msg, b"from async writer");
            reader.close().unwrap();
        });

        write_handle.await.unwrap();
        reader_thread.join().unwrap();
    }

    /// Exact real-world flow: writer created from async, reader.open()
    /// deferred to a plain background thread (as in ZiskStream), writer
    /// sends from spawn_blocking.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_same_blocking_thread_reader_then_writer() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15009".parse().unwrap();

        // Writer created in async context (like ZiskStdin::from_stream in #[tokio::main])
        let mut writer = QuicStreamWriter::new(server_addr).unwrap();

        let (tx, _rx) = mpsc::channel();

        // Reader opens on a plain thread (like ZiskStream's background thread)
        let reader_handle = thread::spawn(move || {
            let mut reader = QuicStreamReader::new(server_addr).unwrap();
            reader.open().unwrap();
            tx.send(()).unwrap(); // signal connected
            let msg = reader.next().unwrap().unwrap();
            assert_eq!(msg, b"from writer");
            reader.close().unwrap();
        });

        // Writer sends from spawn_blocking (like flush())
        tokio::task::spawn_blocking(move || {
            writer.write(b"from writer").unwrap();
            // Wait for reader to receive before closing
            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        })
        .await
        .unwrap();

        reader_handle.join().unwrap();
    }
}
