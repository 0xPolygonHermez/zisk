//! Shared types and algorithms for the ZisK guest and host.

/// Re-export the SHA-256 hasher and its `Digest` trait from the `sha2` crate.
pub use sha2::{Digest, Sha256};

/// Re-export the `hex` crate for encoding/decoding byte slices as hex strings.
pub use hex;

/// A 32-byte array that holds a raw SHA-256 digest.
pub type Hash = [u8; 32];

/// Computes the Merkle root of `leaves` using SHA-256 to hash sibling pairs.
///
/// Odd-length levels duplicate the last leaf before pairing. Reduces in-place
/// until a single root hash remains.
pub fn merkle_root(mut leaves: Vec<Hash>) -> Hash {
    let mut buf = [0u8; 64];
    while leaves.len() > 1 {
        let next_len = leaves.len().div_ceil(2);
        for i in 0..next_len {
            let left = leaves[2 * i];
            let right = leaves.get(2 * i + 1).copied().unwrap_or(left);
            buf[..32].copy_from_slice(&left);
            buf[32..].copy_from_slice(&right);
            leaves[i] = Sha256::digest(&buf).into();
        }
        leaves.truncate(next_len);
    }
    leaves[0]
}

/// Same as [`merkle_root`] but delegates SHA-256 to the ZisK built-in
/// (`ziskos::zisklib::sha256`), which maps to a highly optimised ROM operation
/// inside the zkVM instead of executing the hash circuit in user space.
pub fn merkle_root_zisklib(mut leaves: Vec<Hash>) -> Hash {
    let mut buf = [0u8; 64];
    while leaves.len() > 1 {
        let next_len = leaves.len().div_ceil(2);
        for i in 0..next_len {
            let left = leaves[2 * i];
            let right = leaves.get(2 * i + 1).copied().unwrap_or(left);
            buf[..32].copy_from_slice(&left);
            buf[32..].copy_from_slice(&right);
            leaves[i] = ziskos::zisklib::sha256(&buf);
        }
        leaves.truncate(next_len);
    }
    leaves[0]
}
