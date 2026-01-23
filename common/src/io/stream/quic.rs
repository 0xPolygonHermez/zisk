//! A QUIC-based implementation of StreamReader and StreamWriter.
//! This module provides functionality to read and write data over QUIC connections
//! for both local and network communication.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use quinn::{Connection, Endpoint, ServerConfig};
use tokio::runtime::{Handle, Runtime};

use super::{StreamRead, StreamWrite};

/// Helper to run async code, either using current runtime or creating one
fn run_async<F, T>(f: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    // Try to use current runtime handle if we're already in a tokio context
    match Handle::try_current() {
        Ok(handle) => {
            // We're in a tokio runtime, use block_in_place to allow blocking
            tokio::task::block_in_place(move || handle.block_on(f))
        }
        Err(_) => {
            // Not in a runtime, create a temporary one
            let rt = Runtime::new().context("Failed to create tokio runtime")?;
            rt.block_on(f)
        }
    }
}

/// Ensure crypto provider is initialized (idempotent)
fn ensure_crypto_provider() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// A QUIC implementation of StreamRead that receives data over QUIC streams.
pub struct QuicStreamReader {
    /// The QUIC connection
    connection: Option<Connection>,

    /// Client endpoint
    endpoint: Option<Endpoint>,

    /// Server address to connect to
    server_addr: SocketAddr,
}

impl QuicStreamReader {
    /// Create a new QuicStreamReader that connects to the specified server address.
    ///
    /// This creates a client endpoint that connects to the server to read data.
    pub fn new(server_addr: SocketAddr) -> Result<Self> {
        // Ensure crypto provider is initialized
        ensure_crypto_provider();

        // We don't need to store a runtime anymore since we'll use run_async helper
        Ok(QuicStreamReader { connection: None, endpoint: None, server_addr })
    }
}

impl StreamRead for QuicStreamReader {
    /// Open/initialize the stream for reading
    ///
    /// Establishes a QUIC connection to the server.
    fn open(&mut self) -> Result<()> {
        if self.is_active() {
            return Ok(());
        }

        let server_addr = self.server_addr;
        let (endpoint, connection) = run_async(async move {
            let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;

            // Configure to accept self-signed certificates (for development)
            let rustls_config = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
                .with_no_client_auth();

            let mut client_config = quinn::ClientConfig::new(Arc::new(
                quinn::crypto::rustls::QuicClientConfig::try_from(rustls_config)
                    .map_err(|e| anyhow::anyhow!("Failed to create QUIC client config: {}", e))?,
            ));

            // Configure transport for better performance
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
    ///
    /// Each call to next() accepts a new unidirectional stream and reads
    /// all data from it, providing natural message boundaries.
    fn next(&mut self) -> Result<Option<Vec<u8>>> {
        self.open()?;

        let connection = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("QuicStreamReader: Connection not established"))?
            .clone();

        run_async(async move {
            // Accept next unidirectional stream
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

            // Read all data from the stream (10MB max)
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
            let _ = run_async(async move {
                endpoint.wait_idle().await;
                Ok::<_, anyhow::Error>(())
            });
        }
        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.connection.is_some()
    }
}

/// A QUIC implementation of StreamWrite that sends data over QUIC streams.
pub struct QuicStreamWriter {
    /// The QUIC connection
    connection: Option<Connection>,

    /// Tokio runtime for async operations
    runtime: Arc<Runtime>,

    /// Server endpoint
    endpoint: Option<Endpoint>,

    /// Server address to bind to
    bind_addr: SocketAddr,
}

impl QuicStreamWriter {
    /// Create a new QuicStreamWriter that listens on the specified address.
    ///
    /// This creates a server endpoint that waits for incoming reader connections.
    pub fn new(bind_addr: SocketAddr) -> Result<Self> {
        // Ensure crypto provider is initialized
        ensure_crypto_provider();

        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("Failed to create tokio runtime")?,
        );

        Ok(QuicStreamWriter { connection: None, runtime, endpoint: None, bind_addr })
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
    /// Open/initialize the stream for writing
    ///
    /// Starts listening for incoming reader connections.
    fn open(&mut self) -> Result<()> {
        if self.is_active() {
            return Ok(());
        }

        // Clean up old resources if they exist
        if let Some(endpoint) = self.endpoint.take() {
            self.runtime.block_on(async {
                endpoint.wait_idle().await;
            });
        }

        let server_config = Self::configure_server()?;

        let (endpoint, connection) = self.runtime.block_on(async {
            let endpoint = Endpoint::server(server_config, self.bind_addr)
                .context("Failed to create server endpoint")?;

            // Wait for incoming connection
            let incoming = endpoint.accept().await.context("Failed to accept connection")?;

            let connection = incoming.await.context("Failed to establish connection")?;

            Ok::<_, anyhow::Error>((endpoint, connection))
        })?;

        self.endpoint = Some(endpoint);
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
            .ok_or_else(|| anyhow::anyhow!("QuicStreamWriter: Connection not established"))?;

        let len = item.len();
        let data = item.to_vec();

        self.runtime.block_on(async {
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
        if let Some(endpoint) = self.endpoint.take() {
            self.runtime.block_on(async {
                endpoint.wait_idle().await;
            });
        }
        Ok(())
    }

    /// Check if the stream is currently active
    fn is_active(&self) -> bool {
        self.connection.is_some()
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

    #[test]
    fn test_single_message() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15001".parse().unwrap();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"Hello, QUIC!").unwrap();

            // Wait for reader to finish before closing
            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        // Give writer time to start listening
        thread::sleep(Duration::from_millis(500));

        // Reader connects and reads message
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, b"Hello, QUIC!");
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_multiple_messages() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15002".parse().unwrap();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"First").unwrap();
            writer.write(b"Second message").unwrap();
            writer.write(b"Third message with more data!").unwrap();

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(500));

        // Reader connects and reads messages
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
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
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15003".parse().unwrap();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"ABC").unwrap();
            writer.write(b"DEF").unwrap();

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(500));

        // Reader should receive each message as discrete unit
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
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
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15004".parse().unwrap();

        // Create a large message (1MB - QUIC can handle this)
        let large_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
        let large_data_clone = large_data.clone();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(&large_data).unwrap();

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(500));

        // Reader receives large message
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        let message = reader.next().unwrap().unwrap();
        assert_eq!(message, large_data_clone);
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_connection_close() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15005".parse().unwrap();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"Message").unwrap();

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(500));

        // Reader receives message and closes
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"Message");
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_multiple_concurrent_messages() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15006".parse().unwrap();

        // Spawn writer (server) thread
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            for i in 0..10 {
                writer.write(format!("Message {}", i).as_bytes()).unwrap();
            }

            thread::sleep(Duration::from_millis(200));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(500));

        // Reader receives 10 messages in quick succession
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        for i in 0..10 {
            let msg = reader.next().unwrap().unwrap();
            assert_eq!(msg, format!("Message {}", i).as_bytes());
        }
        reader.close().unwrap();

        writer_thread.join().unwrap();
    }

    #[test]
    fn test_writer_closes_early() {
        init_crypto();
        let server_addr: SocketAddr = "127.0.0.1:15007".parse().unwrap();

        // Spawn writer (server) thread that closes after writing one message
        let writer_thread = thread::spawn(move || {
            let mut writer = QuicStreamWriter::new(server_addr).unwrap();
            writer.write(b"First").unwrap();

            // Writer closes immediately
            thread::sleep(Duration::from_millis(100));
            writer.close().unwrap();
        });

        thread::sleep(Duration::from_millis(500));

        // Reader receives first message successfully
        let mut reader = QuicStreamReader::new(server_addr).unwrap();
        let msg1 = reader.next().unwrap().unwrap();
        assert_eq!(msg1, b"First");

        reader.close().unwrap();
        writer_thread.join().unwrap();
    }
}
