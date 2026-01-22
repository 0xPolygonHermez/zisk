use tracing::warn;

use crate::io::ZiskIO;
use serde::Serialize;

pub struct ZiskNullStdin;

impl ZiskIO for ZiskNullStdin {
    fn read(&mut self) -> Vec<u8> {
        Vec::new()
    }
    fn read_slice(&mut self, _slice: &mut [u8]) {}
    fn read_into(&mut self, _buffer: &mut [u8]) {}
    fn write<T: Serialize>(&mut self, _data: &T) {
        warn!("NullStdin does not support writing");
    }
    fn write_slice(&mut self, _data: &[u8]) {
        warn!("NullStdin does not support writing");
    }
}
