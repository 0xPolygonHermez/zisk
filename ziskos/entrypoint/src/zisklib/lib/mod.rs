//! Core library for guest programs running on the Zisk zkVM.
//!
//! ## Arithmetic
//! - [`bigint`] — Arbitrary-precision integer arithmetic.
//! - [`uint256`] — Low-level 256-bit add, subtract, multiply, divide, and power.
//!
//! ## Hashing
//! - [`blake2b`], [`keccak256`], [`sha256`], [`ripemd160`]
//!
//! ## Elliptic curves
//! - [`secp256k1`], [`secp256r1`], [`bn254`], [`bls12_381`]

mod bigint;
mod blake2b;
mod bls12_381;
mod bn254;
mod constants;
mod keccak256;
mod ripemd160;
mod secp256k1;
mod secp256r1;
mod sha256;
mod sw_impl;
mod uint256;
mod utils;
pub mod zkvm_accelerators;

// For public consumption
pub use bigint::*;
pub use blake2b::*;
pub use bls12_381::*;
pub use bn254::*;
pub use constants::*;
pub use keccak256::*;
pub use ripemd160::*;
pub use secp256k1::*;
pub use secp256r1::*;
pub use sha256::*;
pub use uint256::*;
pub use utils::*;
