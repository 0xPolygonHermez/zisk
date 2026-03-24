//! Shared wire-format helpers for the framed stdin protocol.
//!
//! All stdin implementations use the same on-wire layout per entry:
//!
//! ```text
//! [8-byte LE payload length][payload bytes][zero-padding to 8-byte alignment]
//! ```
//!
//! Centralising these two primitives here guarantees byte-for-byte consistency
//! across the memory, file, and socket backends.

use std::io::Read;

/// Encode one framed entry and return it as a new buffer.
pub(super) fn prepare_frame(payload: &[u8]) -> Vec<u8> {
    let data_len = payload.len();
    let total_len = 8 + data_len;
    let padding = (8 - (total_len & 7)) & 7;
    let mut buf = Vec::with_capacity(total_len + padding);
    buf.extend_from_slice(&data_len.to_le_bytes());
    buf.extend_from_slice(payload);
    buf.resize(buf.len() + padding, 0u8);
    buf
}

/// Append one framed entry to an existing buffer.
pub(super) fn append_frame(buf: &mut Vec<u8>, payload: &[u8]) {
    let data_len = payload.len();
    let total_len = 8 + data_len;
    let padding = (8 - (total_len & 7)) & 7;
    buf.extend_from_slice(&data_len.to_le_bytes());
    buf.extend_from_slice(payload);
    buf.resize(buf.len() + padding, 0u8);
}

/// Read the next framed entry from `r`.
///
/// Returns the raw payload bytes (without the length header or padding).
pub(super) fn read_frame<R: Read>(r: &mut R) -> std::io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 8];
    r.read_exact(&mut len_bytes)?;
    let len = usize::from_le_bytes(len_bytes);

    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;

    // Max padding is 7 bytes; use a stack buffer to avoid heap allocation.
    let total_len = 8 + len;
    let padding = (8 - (total_len & 7)) & 7;
    let mut pad = [0u8; 7];
    r.read_exact(&mut pad[..padding])?;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrip_basic() {
        let buf = prepare_frame(b"hello");
        let got = read_frame(&mut Cursor::new(buf)).unwrap();
        assert_eq!(got, b"hello");
    }

    #[test]
    fn padding_zero_when_aligned() {
        // payload_len = 8 → total = 16, padding = 0
        let buf = prepare_frame(b"12345678");
        assert_eq!(buf.len(), 16);
        let got = read_frame(&mut Cursor::new(buf)).unwrap();
        assert_eq!(got, b"12345678");
    }

    #[test]
    fn padding_seven_bytes() {
        // payload_len = 1 → total = 9, padding = 7
        let buf = prepare_frame(b"X");
        assert_eq!(buf.len(), 16);
        let got = read_frame(&mut Cursor::new(buf)).unwrap();
        assert_eq!(got, b"X");
    }

    #[test]
    fn multiple_entries_sequential() {
        let mut buf = Vec::new();
        append_frame(&mut buf, b"first");
        append_frame(&mut buf, b"second");
        let mut cursor = Cursor::new(buf);
        assert_eq!(read_frame(&mut cursor).unwrap(), b"first");
        assert_eq!(read_frame(&mut cursor).unwrap(), b"second");
    }

    #[test]
    fn empty_payload() {
        let buf = prepare_frame(b"");
        // total = 8, padding = 0
        assert_eq!(buf.len(), 8);
        let got = read_frame(&mut Cursor::new(buf)).unwrap();
        assert_eq!(got, b"");
    }
}
