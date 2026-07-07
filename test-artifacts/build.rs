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

    // Check for b_native feature
    let b_native = env::var("CARGO_FEATURE_B_NATIVE").is_ok();
    if b_native {
        features.push("b_native");
    }

    // Check for b_soft feature
    let b_soft = env::var("CARGO_FEATURE_B_SOFT").is_ok();
    if b_soft {
        features.push("b_soft");
    }

    // Check for mutual exclusivity of features
    if b_native && b_soft {
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "features `b_native` and `b_soft` are mutually exclusive",
        ));
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
