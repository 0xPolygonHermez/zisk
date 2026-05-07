//! Operations on the secp256r1 (P-256, NIST P-256) elliptic curve.
//!
//! - [`curve`] — Point arithmetic (scalar mul, curve membership check).
//! - [`field`] — Base field Fp arithmetic.
//! - [`scalar`] — Scalar field Fn arithmetic.
//! - [`ecdsa`] — ECDSA signature verification.

mod constants;
mod curve;
mod ecdsa;
mod field;
mod scalar;

pub use curve::*;
pub use ecdsa::*;
pub use field::*;
pub use scalar::*;
