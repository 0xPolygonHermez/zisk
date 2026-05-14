use std::{
    io::{Error, Result},
    path::PathBuf,
};

use zisk_sdk::build_program;

fn main() -> Result<()> {
    let programs_path =
        [env!("CARGO_MANIFEST_DIR"), "programs"].iter().collect::<PathBuf>().canonicalize()?;

    println!("programs path: {programs_path:?}");

    build_program(
        programs_path
            .to_str()
            .ok_or_else(|| Error::other(format!("Invalid programs path: {programs_path:?}")))?,
    );

    Ok(())
}