#![no_std]

//! Native Rust verifiers, generated from ZisK's own proving keys.
//!
//! They live here, not in pil2-proofman, because the aggregator binds the
//! application's publics into q_verify: a verifier generated for a different
//! application rejects valid ZisK proofs.
//!
//! Regenerate with `tools/release-artifacts/regen.sh`, which installs each family's
//! `<key>/zisk/<stage>/<stage>.verifier.rs` with the generator's `crate::` paths pointing
//! at `proofman_verifier::`, and its keys with them.

// The generated verifiers allocate and this crate is no_std.
extern crate alloc;

pub mod blake3 {
    pub mod vadcop_final;
    pub mod vadcop_final_compressed;
}

pub mod poseidon1 {
    pub mod vadcop_final;
    pub mod vadcop_final_compressed;
}

pub mod poseidon2 {
    pub mod vadcop_final;
    pub mod vadcop_final_compressed;
}

mod decode;
pub mod keys;
mod verifier;

pub use decode::*;
pub use keys::*;
pub use proofman_verifier::VadcopFinalProof;
pub use verifier::*;
