use crate::io::ZiskIORead;
use anyhow::Result;
use serde::de::DeserializeOwned;

pub struct ZiskNullStdin;

impl ZiskIORead for ZiskNullStdin {
    fn read_raw_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn read_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn read_slice(&self, _slice: &mut [u8]) {}

    fn read<T: DeserializeOwned>(&self) -> Result<T> {
        Err(anyhow::anyhow!("NullStdin does not support reading"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_raw_bytes_is_empty() {
        assert!(ZiskNullStdin.read_raw_bytes().is_empty());
    }

    #[test]
    fn read_bytes_is_empty() {
        assert!(ZiskNullStdin.read_bytes().is_empty());
    }

    #[test]
    fn read_typed_returns_error() {
        assert!(ZiskNullStdin.read::<u64>().is_err());
    }

    #[test]
    fn read_slice_leaves_buffer_unchanged() {
        // ZiskNullStdin::read_slice is a no-op — documents the known behaviour
        // that the caller's buffer is not filled.
        let mut buf = vec![0xFFu8; 4];
        ZiskNullStdin.read_slice(&mut buf);
        assert_eq!(buf, vec![0xFFu8; 4]);
    }
}
