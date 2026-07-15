use std::{
    io::{Error, Result},
    path::PathBuf,
};

use std::env;
use zisk_sdk::{build_program_with_args, BuildArgs};

/// Every guest program under `programs/`, by package name. Each has a matching
/// Cargo feature (see `Cargo.toml`); enabling the feature builds that program and
/// exposes its `ELF_*` constant in `lib.rs`. Keep this list in sync with the
/// `[features]` table and the constants in `src/lib.rs`.
const PROGRAMS: &[&str] = &[
    "add256",
    "agg_verify",
    "arith256",
    "arith256_mod",
    "arith384_mod",
    "big_input",
    "bigint",
    "blake2",
    "bls12_381",
    "bls12_381_add",
    "bls12_381_complex_add",
    "bls12_381_complex_mul",
    "bls12_381_complex_sub",
    "bls12_381_dbl",
    "bn254",
    "bn254_add",
    "bn254_complex_add",
    "bn254_complex_mul",
    "bn254_complex_sub",
    "bn254_dbl",
    "diagnostic",
    "diagnostic-hints",
    "fib_mod",
    "hashes",
    "keccak",
    "liveness",
    "missing_entrypoint",
    "panic_modes",
    "poseidon1",
    "poseidon2",
    "secp256k1",
    "secp256k1_add",
    "secp256k1_dbl",
    "secp256r1",
    "secp256r1_add",
    "secp256r1_dbl",
    "sha256",
    "uint256",
];

/// Whether the Cargo feature matching `program` (its package name) is enabled.
/// Cargo exposes `features."foo-bar"` as the env var `CARGO_FEATURE_FOO_BAR`.
fn feature_enabled(program: &str) -> bool {
    let var = format!("CARGO_FEATURE_{}", program.to_uppercase().replace('-', "_"));
    env::var(var).is_ok()
}

fn main() -> Result<()> {
    let programs_path =
        [env!("CARGO_MANIFEST_DIR"), "programs"].iter().collect::<PathBuf>().canonicalize()?;

    // Build only the programs whose feature is enabled. With the default `all`
    // feature this is every program (the historical behavior); a consumer that
    // opts into a subset builds only those, skipping the cost of the rest.
    let enabled: Vec<String> =
        PROGRAMS.iter().filter(|p| feature_enabled(p)).map(|p| p.to_string()).collect();

    // No program selected (e.g. `default-features = false` with no program
    // feature): nothing to build, and `lib.rs` exposes no constants.
    if enabled.is_empty() {
        return Ok(());
    }

    // Guest-compilation flags, orthogonal to program selection.
    let mut features = Vec::new();
    if env::var("CARGO_FEATURE_BIT_MANIPULATION_EXTENSIONS").is_ok() {
        features.push("bit_manipulation_extensions");
    }

    // Build guests with the same profile as the host so profiling/benchmarks
    // measure an optimized guest. `BuildArgs::default()` is debug, which left
    // every guest ELF unoptimized (uninlined field ops + debug_assertions).
    // Mirrors ziskbuild/src/aggregation.rs's PROFILE handling.
    let release = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
    let mut build_args = BuildArgs::default().release(release);
    build_args.features = if features.is_empty() { None } else { Some(features.join(",")) };
    // Only restrict the build to specific packages for a strict subset.
    if enabled.len() < PROGRAMS.len() {
        build_args.packages = enabled;
    }

    build_program_with_args(
        programs_path
            .to_str()
            .ok_or_else(|| Error::other(format!("Invalid programs path: {programs_path:?}")))?,
        build_args,
    );

    Ok(())
}
