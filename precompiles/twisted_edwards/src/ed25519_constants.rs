#![allow(dead_code)]
//! Constants for the Ed25519 (twisted-Edwards) point-add precompile.
//!
//! Curve (RFC 8032, edwards25519): a*x^2 + y^2 = 1 + d*x^2*y^2 over Fp, p = 2^255 - 19, with
//! a = -1 and d = -121665/121666 mod p.

/// Fp = 2^255 - 19
pub const ED25519_PRIME: &str = "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffed";

/// d = -121665/121666 mod p
pub const ED25519_D: &str = "52036cee2b6ffe738cc740797779e89800700a4d4141d8ab75eb4dca135978a3";

/// `Fp` split into 16 little-endian 16-bit chunks (LSB first).
pub const ED25519_PRIME_CHUNKS: [i64; 16] = [
    0xFFED, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
    0xFFFF, 0xFFFF, 0xFFFF, 0x7FFF,
];
