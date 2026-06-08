//! Codec for the fixed-size stdio request/response frame.
//!
//! The ASM services speak a fixed `[u64; 5]` frame (40 bytes, little-endian)
//! over stdio. This module is the single place that marshals between the typed
//! [`RequestData`]/[`ResponseData`] arrays and their on-wire byte form.

use super::{RequestData, ResponseData};
use anyhow::Result;

/// Encode a request payload into its 40-byte little-endian wire frame.
pub(super) fn encode_request(request: RequestData) -> [u8; 40] {
    let mut buf = [0u8; 40];
    for (i, word) in request.iter().enumerate() {
        buf[i * 8..(i + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }
    buf
}

/// Decode a 40-byte little-endian wire frame into a response payload.
pub(super) fn decode_response(buf: &[u8; 40]) -> Result<ResponseData> {
    let mut response = ResponseData::default();
    for (i, chunk) in buf.chunks_exact(8).enumerate() {
        response[i] = u64::from_le_bytes(chunk.try_into()?);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_each_word_little_endian() {
        let req = [1u64, 0x0203, 0, 0, 0];
        let bytes = encode_request(req);
        assert_eq!(bytes.len(), 40);
        assert_eq!(&bytes[0..8], &1u64.to_le_bytes());
        assert_eq!(&bytes[8..16], &0x0203u64.to_le_bytes());
        assert_eq!(&bytes[16..40], &[0u8; 24]);
    }

    #[test]
    fn encode_then_decode_round_trips() {
        for req in [[0u64; 5], [7, 123, 456, u64::MAX, 1], [1000000, 0, 0, 0, 0]] {
            assert_eq!(decode_response(&encode_request(req)).unwrap(), req);
        }
    }

    #[test]
    fn decode_reads_little_endian() {
        let mut buf = [0u8; 40];
        buf[0..8].copy_from_slice(&0xCAFEu64.to_le_bytes());
        buf[32..40].copy_from_slice(&u64::MAX.to_le_bytes());
        let decoded = decode_response(&buf).unwrap();
        assert_eq!(decoded[0], 0xCAFE);
        assert_eq!(decoded[4], u64::MAX);
    }
}
