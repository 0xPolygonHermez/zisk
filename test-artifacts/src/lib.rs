//! Compiles every program under `programs/` into an ELF and re-exports each one
//! as a constant for use by other crates in the workspace.
//!
//! **Adding a new program:** register it as a member of the `programs/` workspace
//! so it gets built, then expose its ELF via a `load_program!` constant below.

use zisk_sdk::{load_program, GuestProgram};

pub use sha_hasher_common::Output;

pub const ELF_ADD256: GuestProgram = load_program!("add256");
pub const ELF_AGG_VERIFY: GuestProgram = load_program!("agg_verify");
pub const ELF_ARITH256: GuestProgram = load_program!("arith256");
pub const ELF_ARITH256_MOD: GuestProgram = load_program!("arith256_mod");
pub const ELF_ARITH384_MOD: GuestProgram = load_program!("arith384_mod");
pub const ELF_BIG_INPUT: GuestProgram = load_program!("big_input");
pub const ELF_BLAKE2: GuestProgram = load_program!("blake2");
pub const ELF_BLS12_381: GuestProgram = load_program!("bls12_381");
pub const ELF_BLS12_381_ADD: GuestProgram = load_program!("bls12_381_add");
pub const ELF_BLS12_381_COMPLEX_ADD: GuestProgram = load_program!("bls12_381_complex_add");
pub const ELF_BLS12_381_COMPLEX_MUL: GuestProgram = load_program!("bls12_381_complex_mul");
pub const ELF_BLS12_381_COMPLEX_SUB: GuestProgram = load_program!("bls12_381_complex_sub");
pub const ELF_BLS12_381_DBL: GuestProgram = load_program!("bls12_381_dbl");
pub const ELF_BN254: GuestProgram = load_program!("bn254");
pub const ELF_BN254_ADD: GuestProgram = load_program!("bn254_add");
pub const ELF_BN254_COMPLEX_ADD: GuestProgram = load_program!("bn254_complex_add");
pub const ELF_BN254_COMPLEX_MUL: GuestProgram = load_program!("bn254_complex_mul");
pub const ELF_BN254_COMPLEX_SUB: GuestProgram = load_program!("bn254_complex_sub");
pub const ELF_BN254_DBL: GuestProgram = load_program!("bn254_dbl");
pub const ELF_DIAGNOSTIC: GuestProgram = load_program!("diagnostic");
pub const ELF_FIB_MOD: GuestProgram = load_program!("fib_mod");
pub const ELF_KECCAK: GuestProgram = load_program!("keccak");
pub const ELF_LIVENESS: GuestProgram = load_program!("liveness");
pub const ELF_MODEXP: GuestProgram = load_program!("modexp");
pub const ELF_PANIC_MODES: GuestProgram = load_program!("panic_modes");
pub const ELF_POSEIDON2: GuestProgram = load_program!("poseidon2");
pub const ELF_SECP256K1: GuestProgram = load_program!("secp256k1");
pub const ELF_SECP256K1_ADD: GuestProgram = load_program!("secp256k1_add");
pub const ELF_SECP256K1_DBL: GuestProgram = load_program!("secp256k1_dbl");
pub const ELF_SECP256R1: GuestProgram = load_program!("secp256r1");
pub const ELF_SECP256R1_ADD: GuestProgram = load_program!("secp256r1_add");
pub const ELF_SECP256R1_DBL: GuestProgram = load_program!("secp256r1_dbl");
pub const ELF_SHA256: GuestProgram = load_program!("sha256");
pub const ELF_SHA_HASHER: GuestProgram = load_program!("sha_hasher");
pub const ELF_UINT256: GuestProgram = load_program!("uint256");
