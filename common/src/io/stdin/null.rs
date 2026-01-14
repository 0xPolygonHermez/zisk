use tracing::warn;

use crate::io::ZiskIO;

pub struct ZiskNullStdin;

impl ZiskIO for ZiskNullStdin {
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
