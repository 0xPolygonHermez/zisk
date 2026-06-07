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
