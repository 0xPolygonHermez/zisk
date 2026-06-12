//! Shared types and algorithms for the ZisK guest and host.

/// Re-export the SHA-256 hasher and its `Digest` trait from the `sha2` crate.
pub use sha2::{Digest, Sha256};

/// Re-export the `hex` crate for encoding/decoding hex strings.
pub use hex;

/// A 32-byte array that holds a raw SHA-256 digest.
pub type Hash = [u8; 32];

/// Output committed by the guest: the starting value and the full Collatz sequence.
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct OutputDTO {
    pub n: u64,
    pub sequence: Vec<u64>,
}

/// Returns the Collatz sequence starting from `n`, ending at 1.
pub fn collatz(mut n: u64) -> Vec<u64> {
    let mut seq = vec![n];
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        seq.push(n);
    }
    seq
}
