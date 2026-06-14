//! Shared types and re-exports for the hash example.

/// Re-export the SHA-256 hasher and its `Digest` trait from the `sha2` crate.
pub use sha2::{Digest, Sha256};

/// Re-export the `hex` crate for encoding/decoding hex strings.
pub use hex;

/// A 32-byte array that holds a raw SHA-256 digest.
pub type Hash = [u8; 32];
