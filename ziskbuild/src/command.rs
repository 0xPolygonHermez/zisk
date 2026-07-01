use crate::{BuildArgs, HELPER_TARGET_SUBDIR, ZISK_LINKER_SCRIPT, ZISK_TARGET};
use anyhow::{Context, Result};
use cargo_metadata::camino::Utf8PathBuf;
use std::{path::PathBuf, process::Command};

/// Get the command to build the program locally.
pub(crate) fn create_command(
    args: &BuildArgs,
    program_dir: &Utf8PathBuf,
    program_metadata: &cargo_metadata::Metadata,
) -> Result<Command> {
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

    for package in &args.packages {
        command.args(["--package", package]);
    }
    for bin in &args.binaries {
        command.args(["--bin", bin]);
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
            .map_err(|_| {
                anyhow::anyhow!(
                    "ZisK toolchain '{}' is not installed or rustup is not available.\n\
                     Run `cargo zisk toolchain install` to install it.",
                    crate::RUSTUP_TOOLCHAIN_NAME
                )
            })?;

        if !output.status.success() {
            anyhow::bail!(
                "ZisK toolchain '{}' is not installed.\n\
                 Run `cargo zisk toolchain install` to install it.",
                crate::RUSTUP_TOOLCHAIN_NAME
            );
        }

        let stdout_string =
            String::from_utf8(output.stdout).context("Can't parse rustc --print sysroot stdout")?;

        PathBuf::from(stdout_string.trim()).join("bin/rustc")
    };

    command
        .env_remove("RUSTC")
        .env("RUSTC", rustc_bin.display().to_string())
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");

    // Inject the zisk linker script

    // Write the linker script to a uniquely-named temp file. A predictable
    // path (`$TMPDIR/zisk.ld`) can race across concurrent invocations and is
    // exposed to temp-file symlink attacks. Keep the handle alive until cargo
    // finishes so the file is not removed while the linker still needs it.

    let mut linker_script = tempfile::Builder::new()
        .prefix("zisk-")
        .suffix(".ld")
        .tempfile()
        .context("Failed to create temporary Zisk linker script")?;

    std::io::Write::write_all(&mut linker_script, ZISK_LINKER_SCRIPT)
        .context("Failed to write Zisk linker script to temp file")?;

    // Set linker script flag and zisk_guest cfg to RUSTFLAGS
    let rust_flags = format!("--cfg zisk_guest -C link-arg=-T{}", linker_script.path().display())
        .trim()
        .to_string();
    command.env("RUSTFLAGS", rust_flags);

    let canonicalized_program_dir =
        program_dir.canonicalize().context("Failed to canonicalize program directory")?;
    command.current_dir(canonicalized_program_dir);

    // Use a separate subdirectory to avoid conflicts with the host build
    command.env("CARGO_TARGET_DIR", program_metadata.target_directory.join(HELPER_TARGET_SUBDIR));

    Ok(command)
}
