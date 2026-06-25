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

    let mut build_args = BuildArgs::default();
    build_args.features = if features.is_empty() { None } else { Some(features.join(",")) };

    build_program_with_args(
        programs_path
            .to_str()
            .ok_or_else(|| Error::other(format!("Invalid programs path: {programs_path:?}")))?,
        build_args,
    );

    Ok(())
}
