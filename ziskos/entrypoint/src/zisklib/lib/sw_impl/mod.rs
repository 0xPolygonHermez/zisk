//! Pure-software fallback implementations for zkVM accelerators (non-hints, non-zkVM builds only).
#[cfg(all(not(feature = "hints"), not(zisk_guest)))]
pub mod blake2;
#[cfg(all(not(feature = "hints"), not(zisk_guest)))]
pub mod bls12;
#[cfg(all(not(feature = "hints"), not(zisk_guest)))]
pub mod bn254;
#[cfg(all(not(feature = "hints"), not(zisk_guest)))]
pub mod modexp;
#[cfg(all(not(feature = "hints"), not(zisk_guest)))]
pub mod ripemd160;
#[cfg(all(not(feature = "hints"), not(zisk_guest)))]
pub mod secp256k1;
#[cfg(all(not(feature = "hints"), not(zisk_guest)))]
pub mod sha256;
