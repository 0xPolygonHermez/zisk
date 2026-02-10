use tracing::warn;

use crate::io::ZiskIO;
use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

pub struct ZiskNullStdin;

impl ZiskIO for ZiskNullStdin {
    fn read_bytes(&self) -> Vec<u8> {
        Vec::new()
    }
    fn read_slice(&self, _slice: &mut [u8]) {}
    fn read_into(&self, _buffer: &mut [u8]) {}
    fn read<T: DeserializeOwned>(&self) -> Result<T> {
        Err(anyhow::anyhow!("NullStdin does not support reading"))
    }
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
