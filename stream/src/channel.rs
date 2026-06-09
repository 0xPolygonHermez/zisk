use std::sync::mpsc;

use crate::StreamRead;

/// A stream reader that reads byte chunks from an [`mpsc`] channel.
///
/// Used by the coordinator to feed gRPC-pushed hints data into the
/// `PrecompileHintsRelay` without opening a network URI.  The paired
/// [`mpsc::Sender`] is held by the coordinator; dropping it (or sending
/// `None`) signals EOF to the relay thread.
pub struct ChannelStreamReader {
    rx: mpsc::Receiver<Option<Vec<u8>>>,
    done: bool,
}

impl ChannelStreamReader {
    /// Create a linked (reader, sender) pair.
    ///
    /// The returned `Sender` should be stored by the coordinator and used to
    /// push raw hint bytes.  Send `None` (or simply drop the sender) to signal
    /// end-of-stream.
    pub fn new_pair() -> (Self, mpsc::Sender<Option<Vec<u8>>>) {
        let (tx, rx) = mpsc::channel();
        (Self { rx, done: false }, tx)
    }
}

impl StreamRead for ChannelStreamReader {
    fn open(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn next(&mut self) -> anyhow::Result<Option<Vec<u8>>> {
        if self.done {
            return Ok(None);
        }
        match self.rx.recv() {
            Ok(Some(data)) => Ok(Some(data)),
            Ok(None) => {
                self.done = true;
                Ok(None)
            }
            Err(_) => {
                // Sender dropped — treat as EOF.
                self.done = true;
                Ok(None)
            }
        }
    }

    fn close(&mut self) -> anyhow::Result<()> {
        self.done = true;
        Ok(())
    }

    fn is_active(&self) -> bool {
        !self.done
    }
}
