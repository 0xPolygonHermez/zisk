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
    "dma_mt",
    "fib_mod",
    "hashes",
    "keccak",
    "keccakf_cache",
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

    // Restrict a subset build to the requested guests by binary name (`--bin`),
    // not by package (`--package`). Some guest names collide with crates.io
    // dependency package names in `programs/` (e.g. `keccak`, `secp256k1`), which
    // makes `--package <name>` ambiguous; `--bin <name>` is unambiguous because
    // only the guest defines a binary of that name. With the default `all` feature
    // every program is enabled and we build the whole workspace with no target
    // selection (the historical behavior).
    let subset = enabled.len() < PROGRAMS.len();

    // `bit_manipulation_extensions` exists only on the `diagnostic` guest, so
    // forward it scoped to that package (`diagnostic/...`) and only in a subset
    // build that includes `diagnostic` (so `--bin diagnostic` is in the build).
    // Forwarding it otherwise — the full build, or a subset without `diagnostic` —
    // would make Cargo error, so it has no effect in those cases.
    let mut features = Vec::new();

    // Check for zba feature
    let zba = env::var("CARGO_FEATURE_ZBA").is_ok();
    if zba {
        features.push("zba");
    }

    // Check for zba_native feature
    let zba_native = env::var("CARGO_FEATURE_ZBA_NATIVE").is_ok();
    if zba_native {
        features.push("zba_native");
    }

    // Check for zbc feature
    let zbc = env::var("CARGO_FEATURE_ZBC").is_ok();
    if zbc {
        features.push("zbc");
    }

    // Check for zbc_native feature
    let zbc_native = env::var("CARGO_FEATURE_ZBC_NATIVE").is_ok();
    if zbc_native {
        features.push("zbc_native");
    }

    // Check for zbkc feature
    let zbkc = env::var("CARGO_FEATURE_ZBKC").is_ok();
    if zbkc {
        features.push("zbkc");
    }

    // Check for zbkc_native feature
    let zbkc_native = env::var("CARGO_FEATURE_ZBKC_NATIVE").is_ok();
    if zbkc_native {
        features.push("zbkc_native");
    }

    // Check for zbkx feature
    let zbkx = env::var("CARGO_FEATURE_ZBKX").is_ok();
    if zbkx {
        features.push("zbkx");
    }

    // Check for zbkx_native feature
    let zbkx_native = env::var("CARGO_FEATURE_ZBKX_NATIVE").is_ok();
    if zbkx_native {
        features.push("zbkx_native");
    }

    // Check for zicond_native feature
    let zicond_native = env::var("CARGO_FEATURE_ZICOND_NATIVE").is_ok();
    if zicond_native {
        features.push("zicond_native");
    }

    // Build guests with the same profile as the host so profiling/benchmarks
    // measure an optimized guest. `BuildArgs::default()` is debug, which left
    // every guest ELF unoptimized (uninlined field ops + debug_assertions).
    // Mirrors ziskbuild/src/aggregation.rs's PROFILE handling.
    let release = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
    let mut build_args = BuildArgs::default().release(release);
    build_args.features = if features.is_empty() { None } else { Some(features.join(",")) };
    if subset {
        build_args.binaries = enabled;
    }

    build_program_with_args(
        programs_path
            .to_str()
            .ok_or_else(|| Error::other(format!("Invalid programs path: {programs_path:?}")))?,
        build_args,
    );

    Ok(())
}
