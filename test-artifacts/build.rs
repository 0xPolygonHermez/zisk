use std::{
    io::{Error, Result},
    path::PathBuf,
};

use std::env;
use zisk_sdk::{build_program_with_args, BuildArgs};

fn main() -> Result<()> {
    let programs_path =
        [env!("CARGO_MANIFEST_DIR"), "programs"].iter().collect::<PathBuf>().canonicalize()?;

    // Collect enabled features from the environment
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

    // Build guests with the same profile as the host so profiling/benchmarks
    // measure an optimized guest. `BuildArgs::default()` is debug, which left
    // every guest ELF unoptimized (uninlined field ops + debug_assertions).
    // Mirrors ziskbuild/src/aggregation.rs's PROFILE handling.
    let release = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
    let mut build_args = BuildArgs::default().release(release);
    build_args.features = if features.is_empty() { None } else { Some(features.join(",")) };

    build_program_with_args(
        programs_path
            .to_str()
            .ok_or_else(|| Error::other(format!("Invalid programs path: {programs_path:?}")))?,
        build_args,
    );

    Ok(())
}
