use tracing::warn;

use crate::io::ZiskIO;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

pub struct ZiskNullStdin;

impl ZiskIO for ZiskNullStdin {
    fn read(&self) -> Vec<u8> {
        Vec::new()
    }
    fn read_slice(&self, _slice: &mut [u8]) {}
    fn read_into(&self, _buffer: &mut [u8]) {}
    fn write<T: Serialize>(&self, _data: &T) {
        warn!("NullStdin does not support writing");
    }
    fn write_slice(&self, _data: &[u8]) {
        warn!("NullStdin does not support writing");
    }
    fn save(&self, _path: &Path) -> Result<()> {
        warn!("NullStdin does not support saving");
        Ok(())
    }
}
