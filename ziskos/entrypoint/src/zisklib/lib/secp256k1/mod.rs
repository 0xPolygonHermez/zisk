//! Operations on the secp256k1 (K-256) elliptic curve.
//! Specified in Certicom’s SECG in [SEC 2: Recommended Elliptic Curve Domain Parameters](https://www.secg.org/sec2-v2.pdf).
//!
//! - [`curve`] — Point arithmetic (add, scalar mul, multi-scalar mul, lift-x).
//! - [`glv`] — GLV endomorphism for accelerating scalar multiplication.
//! - [`field`] — Base field Fp arithmetic.
//! - [`scalar`] — Scalar field Fn arithmetic.
//! - [`ecdsa`] — ECDSA signature verification and public-key recovery.
//! - [`schnorr`] — Schnorr signature verification.

mod constants;
mod curve;
mod ecdsa;
mod field;
mod glv;
mod scalar;
mod schnorr;

pub use curve::*;
pub use ecdsa::*;
pub use field::*;
pub use glv::*;
pub use scalar::*;
pub use schnorr::*;
