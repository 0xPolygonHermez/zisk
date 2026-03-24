use crate::{BuildArgs, HELPER_TARGET_SUBDIR, ZISK_TARGET};
use cargo_metadata::camino::Utf8PathBuf;
use std::{path::PathBuf, process::Command};

/// Get the command to build the program locally.
pub(crate) fn create_command(
    args: &BuildArgs,
    program_dir: &Utf8PathBuf,
    program_metadata: &cargo_metadata::Metadata,
) -> Command {
    // Construct the cargo run command
    let mut command = Command::new("cargo");
    command.args(["+zisk", "build"]);
    // Add the feature selection flags
    if let Some(features) = &args.features {
        command.arg("--features").arg(features);
    }
    if args.all_features {
        command.arg("--all-features");
    }

    if args.no_default_features {
        command.arg("--no-default-features");
    }
    if args.release {
        command.arg("--release");
    }

    command.args(["--target", ZISK_TARGET]);

    // Set up the command to inherit the parent's stdout and stderr
    // command.stdout(Stdio::inherit());
    // command.stderr(Stdio::inherit());

    // // Execute the command
    // let status = command.status().context("Failed to execute cargo build command")?;
    // if !status.success() {
    //     return Err(anyhow!("Cargo run command failed with status {}", status));
    // }

    let rustc_bin = {
        let output = Command::new("rustc")
            .env("RUSTUP_TOOLCHAIN", crate::RUSTUP_TOOLCHAIN_NAME)
            .arg("--print")
            .arg("sysroot")
            .output()
            .expect("rustc --print sysroot should succeed");

        let stdout_string =
            String::from_utf8(output.stdout).expect("Can't parse rustc --print rustc stdout");

        PathBuf::from(stdout_string.trim()).join("bin/rustc")
    };

    command.env_remove("RUSTC").env("RUSTC", rustc_bin.display().to_string());

    let canonicalized_program_dir =
        program_dir.canonicalize().expect("Failed to canonicalize program directory");
    command.current_dir(canonicalized_program_dir);

    // Use a separate subdirectory to avoid conflicts with the host build
    command.env("CARGO_TARGET_DIR", program_metadata.target_directory.join(HELPER_TARGET_SUBDIR));

    command
}
