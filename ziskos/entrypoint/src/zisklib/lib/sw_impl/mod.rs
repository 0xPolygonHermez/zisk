//! Pure-software fallback implementations for zkVM accelerators (non-hints, non-zkVM builds only).
#[cfg(all(not(feature = "hints"), not(all(target_os = "zkvm", target_vendor = "zisk"))))]
pub mod blake2;
#[cfg(all(not(feature = "hints"), not(all(target_os = "zkvm", target_vendor = "zisk"))))]
pub mod bls12;
#[cfg(all(not(feature = "hints"), not(all(target_os = "zkvm", target_vendor = "zisk"))))]
pub mod bn254;
#[cfg(all(not(feature = "hints"), not(all(target_os = "zkvm", target_vendor = "zisk"))))]
pub mod modexp;
#[cfg(all(not(feature = "hints"), not(all(target_os = "zkvm", target_vendor = "zisk"))))]
pub mod ripemd160;
#[cfg(all(not(feature = "hints"), not(all(target_os = "zkvm", target_vendor = "zisk"))))]
pub mod secp256k1;
#[cfg(all(not(feature = "hints"), not(all(target_os = "zkvm", target_vendor = "zisk"))))]
pub mod sha256;
