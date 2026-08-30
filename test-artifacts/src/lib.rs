//! Compiles the guest programs under `programs/` into ELFs and re-exports each one
//! as a constant for use by other crates in the workspace.
//!
//! **Feature-gated:** each program has a Cargo feature named after its package
//! (all enabled by default via the `all` feature). A consumer that only needs a
//! subset can set `default-features = false` and enable just the programs it uses
//! — `build.rs` then compiles only those guests and only their constants below
//! are defined:
//!
//! ```toml
//! test-artifacts = { workspace = true, default-features = false, features = ["fib_mod"] }
//! ```
//!
//! **Adding a new program:** register it as a member of the `programs/` workspace,
//! then add a matching feature (to `[features]` and the `all` list in `Cargo.toml`
//! and to `PROGRAMS` in `build.rs`) and expose its ELF via a feature-gated
//! `load_program!` constant below.

#[allow(unused_imports)]
use zisk_sdk::{load_program, GuestProgram};

#[cfg(feature = "add256")]
pub const ELF_ADD256: GuestProgram = load_program!("add256");
#[cfg(feature = "agg_verify")]
pub const ELF_AGG_VERIFY: GuestProgram = load_program!("agg_verify");
#[cfg(feature = "arith256")]
pub const ELF_ARITH256: GuestProgram = load_program!("arith256");
#[cfg(feature = "arith256_mod")]
pub const ELF_ARITH256_MOD: GuestProgram = load_program!("arith256_mod");
#[cfg(feature = "arith384_mod")]
pub const ELF_ARITH384_MOD: GuestProgram = load_program!("arith384_mod");
#[cfg(feature = "big_input")]
pub const ELF_BIG_INPUT: GuestProgram = load_program!("big_input");
#[cfg(feature = "blake2")]
pub const ELF_BLAKE2: GuestProgram = load_program!("blake2");

#[cfg(feature = "blake2s")]
pub const ELF_BLAKE2S: GuestProgram = load_program!("blake2s");
#[cfg(feature = "bls12_381")]
pub const ELF_BLS12_381: GuestProgram = load_program!("bls12_381");
#[cfg(feature = "bls12_381_add")]
pub const ELF_BLS12_381_ADD: GuestProgram = load_program!("bls12_381_add");
#[cfg(feature = "bls12_381_complex_add")]
pub const ELF_BLS12_381_COMPLEX_ADD: GuestProgram = load_program!("bls12_381_complex_add");
#[cfg(feature = "bls12_381_complex_mul")]
pub const ELF_BLS12_381_COMPLEX_MUL: GuestProgram = load_program!("bls12_381_complex_mul");
#[cfg(feature = "bls12_381_complex_sub")]
pub const ELF_BLS12_381_COMPLEX_SUB: GuestProgram = load_program!("bls12_381_complex_sub");
#[cfg(feature = "bls12_381_dbl")]
pub const ELF_BLS12_381_DBL: GuestProgram = load_program!("bls12_381_dbl");
#[cfg(feature = "bn254")]
pub const ELF_BN254: GuestProgram = load_program!("bn254");
#[cfg(feature = "bn254_add")]
pub const ELF_BN254_ADD: GuestProgram = load_program!("bn254_add");
#[cfg(feature = "bn254_complex_add")]
pub const ELF_BN254_COMPLEX_ADD: GuestProgram = load_program!("bn254_complex_add");
#[cfg(feature = "bn254_complex_mul")]
pub const ELF_BN254_COMPLEX_MUL: GuestProgram = load_program!("bn254_complex_mul");
#[cfg(feature = "bn254_complex_sub")]
pub const ELF_BN254_COMPLEX_SUB: GuestProgram = load_program!("bn254_complex_sub");
#[cfg(feature = "bn254_dbl")]
pub const ELF_BN254_DBL: GuestProgram = load_program!("bn254_dbl");
#[cfg(feature = "diagnostic")]
pub const ELF_DIAGNOSTIC: GuestProgram = load_program!("diagnostic");
#[cfg(feature = "diagnostic-hints")]
pub const ELF_DIAGNOSTIC_HINTS: GuestProgram = load_program!("diagnostic-hints");
#[cfg(feature = "fib_mod")]
pub const ELF_FIB_MOD: GuestProgram = load_program!("fib_mod");
#[cfg(feature = "hashes")]
pub const ELF_HASHES: GuestProgram = load_program!("hashes");
#[cfg(feature = "keccak")]
pub const ELF_KECCAK: GuestProgram = load_program!("keccak");
#[cfg(feature = "keccakf_cache")]
pub const ELF_KECCAKF_CACHE: GuestProgram = load_program!("keccakf_cache");
#[cfg(feature = "liveness")]
pub const ELF_LIVENESS: GuestProgram = load_program!("liveness");
#[cfg(feature = "missing_entrypoint")]
pub const ELF_MISSING_ENTRYPOINT: GuestProgram = load_program!("missing_entrypoint");
#[cfg(feature = "bigint")]
pub const ELF_BIGINT: GuestProgram = load_program!("bigint");
#[cfg(feature = "panic_modes")]
pub const ELF_PANIC_MODES: GuestProgram = load_program!("panic_modes");
#[cfg(feature = "poseidon1")]
pub const ELF_POSEIDON1: GuestProgram = load_program!("poseidon1");
#[cfg(feature = "poseidon2")]
pub const ELF_POSEIDON2: GuestProgram = load_program!("poseidon2");
#[cfg(feature = "secp256k1")]
pub const ELF_SECP256K1: GuestProgram = load_program!("secp256k1");
#[cfg(feature = "secp256k1_add")]
pub const ELF_SECP256K1_ADD: GuestProgram = load_program!("secp256k1_add");
#[cfg(feature = "secp256k1_dbl")]
pub const ELF_SECP256K1_DBL: GuestProgram = load_program!("secp256k1_dbl");
#[cfg(feature = "secp256r1")]
pub const ELF_SECP256R1: GuestProgram = load_program!("secp256r1");
#[cfg(feature = "secp256r1_add")]
pub const ELF_SECP256R1_ADD: GuestProgram = load_program!("secp256r1_add");
#[cfg(feature = "secp256r1_dbl")]
pub const ELF_SECP256R1_DBL: GuestProgram = load_program!("secp256r1_dbl");
#[cfg(feature = "sha256")]
pub const ELF_SHA256: GuestProgram = load_program!("sha256");
#[cfg(feature = "uint256")]
pub const ELF_UINT256: GuestProgram = load_program!("uint256");
