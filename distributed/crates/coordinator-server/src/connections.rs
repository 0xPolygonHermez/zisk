//! The API's open connections, counted by client address in `coordinator_api_connections{ip}`.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tonic::transport::server::{Connected, TcpConnectInfo, TcpIncoming};

/// The API's incoming connections on `addr`, bound as tonic's `serve_with_shutdown` binds them
/// by default (`TCP_NODELAY` on, no TCP keepalive), each counted while it is open.
pub(crate) fn incoming(
    addr: SocketAddr,
) -> io::Result<impl Stream<Item = io::Result<CountedConnection>>> {
    let incoming = TcpIncoming::bind(addr)?.with_nodelay(Some(true));
    Ok(incoming.map(|connection| connection.map(CountedConnection::new)))
}

/// A connection to the API, counted in `coordinator_api_connections{ip}` while it is open.
pub(crate) struct CountedConnection {
    inner: TcpStream,
    ip: Option<String>,
}

impl CountedConnection {
    fn new(inner: TcpStream) -> Self {
        let ip = inner.peer_addr().ok().map(|peer| client_ip(peer.ip()).to_string());
        if let Some(ip) = &ip {
            metrics::gauge!("coordinator_api_connections", "ip" => ip.clone()).increment(1.0);
        }
        Self { inner, ip }
    }
}

impl Drop for CountedConnection {
    fn drop(&mut self) {
        if let Some(ip) = self.ip.take() {
            metrics::gauge!("coordinator_api_connections", "ip" => ip).decrement(1.0);
        }
    }
}

/// An IPv4 client reaching a dual-stack socket shows as `::ffff:a.b.c.d`: count it as `a.b.c.d`.
fn client_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        IpAddr::V4(_) => ip,
    }
}

impl Connected for CountedConnection {
    type ConnectInfo = TcpConnectInfo;

    fn connect_info(&self) -> Self::ConnectInfo {
        self.inner.connect_info()
    }
}

impl AsyncRead for CountedConnection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for CountedConnection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn an_ipv4_client_on_a_dual_stack_socket_counts_as_ipv4() {
        let mapped = IpAddr::V6(Ipv4Addr::new(78, 46, 220, 184).to_ipv6_mapped());
        assert_eq!(client_ip(mapped), IpAddr::V4(Ipv4Addr::new(78, 46, 220, 184)));
        assert_eq!(client_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)), IpAddr::V6(Ipv6Addr::LOCALHOST));
    }
}
