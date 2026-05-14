//! Compiles every program under `programs/` into an ELF and re-exports each one
//! as a constant for use by other crates in the workspace.
//!
//! **Adding a new program:** register it as a member of the `programs/` workspace
//! so it gets built, then expose its ELF via a `load_program!` constant below.

use zisk_sdk::{load_program, GuestProgram};

pub const ELF_BLAKE2: GuestProgram = load_program!("blake2");
pub const ELF_BLS12_381: GuestProgram = load_program!("bls12_381");
pub const ELF_BN254: GuestProgram = load_program!("bn254");
pub const ELF_DIAGNOSTIC: GuestProgram = load_program!("diagnostic");
pub const ELF_KECCAK: GuestProgram = load_program!("keccak");
pub const ELF_MODEXP: GuestProgram = load_program!("modexp");
pub const ELF_POSEIDON2: GuestProgram = load_program!("poseidon2");
pub const ELF_SECP256K1: GuestProgram = load_program!("secp256k1");
pub const ELF_SECP256R1: GuestProgram = load_program!("secp256r1");
pub const ELF_SHA256: GuestProgram = load_program!("sha256");
pub const ELF_UINT256: GuestProgram = load_program!("uint256");
