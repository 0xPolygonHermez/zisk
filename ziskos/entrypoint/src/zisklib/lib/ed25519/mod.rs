//! Operations on the Ed25519 (edwards25519) twisted-Edwards curve.
//! Specified in [RFC 8032](https://datatracker.ietf.org/doc/html/rfc8032).
//!
//! - [`curve`] — Point arithmetic over the `ed25519_add` precompile (add, (de)compression,
//!   double scalar multiplication with the base point).
//! - [`field`] — Base field Fp arithmetic (p = 2^255 - 19), square roots of ratios.
//! - [`scalar`] — Scalar reduction mod L.
//! - [`eddsa`] — Ed25519 signature verification (`ed25519-dalek` `verify` / `verify_strict`).

mod constants;
mod curve;
mod eddsa;
mod field;
mod scalar;

#[cfg(all(test, not(feature = "hints")))]
#[rustfmt::skip]
mod test_vectors;
#[cfg(all(test, not(feature = "hints")))]
mod tests;

pub use curve::*;
pub use eddsa::*;
pub use field::*;
pub use scalar::*;
