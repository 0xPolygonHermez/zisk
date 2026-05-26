use crate::input_stream::ZiskStream;
use crate::stdin::ZiskStdin;

/// Describes the source of input data for a ZisK job.
///
/// `Stdin` carries a fully-buffered [`ZiskStdin`] (memory / file).
/// `Stream` carries a [`ZiskStream`] backed by a live transport
/// (QUIC, Unix socket, gRPC) that supports write-after-run.
pub enum InputSource {
    Stdin(ZiskStdin),
    Stream(ZiskStream),
}

impl From<ZiskStdin> for InputSource {
    fn from(s: ZiskStdin) -> Self {
        InputSource::Stdin(s)
    }
}

impl From<&ZiskStdin> for InputSource {
    fn from(s: &ZiskStdin) -> Self {
        InputSource::Stdin(s.clone())
    }
}

impl From<ZiskStream> for InputSource {
    fn from(s: ZiskStream) -> Self {
        InputSource::Stream(s)
    }
}

impl From<&ZiskStream> for InputSource {
    fn from(s: &ZiskStream) -> Self {
        InputSource::Stream(s.clone())
    }
}
