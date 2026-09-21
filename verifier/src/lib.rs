#![no_std]

//! Native Rust verifiers, generated from ZisK's own proving keys.
//!
//! They live here, not in pil2-proofman, because the aggregator binds the
//! application's publics into q_verify: a verifier generated for a different
//! application rejects valid ZisK proofs.
//!
//! Regenerate from `<key>/zisk/<stage>/<stage>.verifier.rs`, rewriting the
//! `crate::` references the generator emits to `proofman_verifier::`.

// The generated verifiers allocate and this crate is no_std.
extern crate alloc;

/// blake3 keys. No compressed stage: blake3 does not build one.
pub mod blake3 {
    pub mod vadcop_final;
}

pub mod poseidon1 {
    pub mod vadcop_final;
    pub mod vadcop_final_compressed;
}

pub mod poseidon2 {
    pub mod vadcop_final;
    pub mod vadcop_final_compressed;
}

mod verifier;

pub use verifier::*;
