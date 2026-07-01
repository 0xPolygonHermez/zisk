//! Operations on the BN254 (alt_bn128) pairing-friendly elliptic curve.
//!
//! ## Point arithmetic
//! - [`curve`] — Point arithmetic for G1.
//! - [`twist`] — Point arithmetic for G2.
//!
//! ## Field arithmetic
//! - [`fp`] — Base field Fp (256-bit prime field).
//! - [`fr`] — Scalar field Fr.
//! - [`fp2`] — Degree-2 extension Fp2.
//! - [`fp6`] — Degree-6 extension Fp6.
//! - [`fp12`] — Degree-12 extension Fp12.
//!
//! ## Pairing
//! - [`miller_loop`] — Miller loop computation.
//! - [`final_exp`] — Final exponentiation.
//! - [`cyclotomic`] — Cyclotomic subgroup arithmetic.
//! - [`pairing`] — Optimal Ate pairing and batch pairing check.

mod constants;
mod curve;
mod cyclotomic;
mod final_exp;
mod fp;
mod fp12;
mod fp2;
mod fp6;
mod fr;
mod miller_loop;
mod pairing;
mod twist;

pub use curve::*;
pub use cyclotomic::*;
pub use final_exp::*;
pub use fp::*;
pub use fp12::*;
pub use fp2::*;
pub use fp6::*;
pub use fr::*;
pub use pairing::*;
pub use twist::*;
