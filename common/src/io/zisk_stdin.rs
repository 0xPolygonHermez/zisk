use tracing::warn;

pub trait ZiskStdin: Send + Sync {
    /// Read a value from the buffer.
    fn read(&mut self) -> Vec<u8>;

    /// Read a slice of bytes from the buffer.
    fn read_slice(&mut self, slice: &mut [u8]);

    /// Read bytes into the provided buffer.
    fn read_into(&mut self, buffer: &mut [u8]);

    /// Write a serialized value to the buffer.
    fn write_serialized(&mut self, data: &[u8]);

    /// Write a slice of bytes to the buffer.
    fn write_bytes(&mut self, data: &[u8]);
}

pub struct NullStdin;

impl ZiskStdin for NullStdin {
    fn read(&mut self) -> Vec<u8> {
        Vec::new()
    }
    fn read_slice(&mut self, _slice: &mut [u8]) {}
    fn read_into(&mut self, _buffer: &mut [u8]) {}
    fn write_serialized(&mut self, _data: &[u8]) {
        warn!("NullStdin does not support writing");
    }
    fn write_bytes(&mut self, _data: &[u8]) {
        warn!("NullStdin does not support writing");
    }
}
