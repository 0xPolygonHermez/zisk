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

    // Check for bit_manipulation_extensions feature
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

    build_program_with_args(
        programs_path
            .to_str()
            .ok_or_else(|| Error::other(format!("Invalid programs path: {programs_path:?}")))?,
        build_args,
    );

    Ok(())
}
