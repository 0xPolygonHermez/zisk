use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};
use zisk_build::{HELPER_TARGET_SUBDIR, ZISK_TARGET, ZISK_VERSION_MESSAGE};

// Structure representing the 'build' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Build the program to a RISC-V ELF file using the ZisK toolchain
pub(crate) struct BuildCmd {
    /// Space or comma separated list of features to activate
    #[arg(short = 'F', long)]
    features: Option<String>,

    /// Activate all available features
    #[arg(long)]
    all_features: bool,

    /// Build artifacts in release mode, with optimizations
    #[arg(long)]
    release: bool,

    /// Do not activate the `default` feature
    #[arg(long)]
    no_default_features: bool,

    /// Copy final artifacts to this directory
    #[arg(long)]
    artifact_dir: Option<String>,

    /// Build only the specified binary (repeat for multiple)
    #[arg(long = "bin", value_name = "BIN")]
    binaries: Vec<String>,

    /// Build only the specified package (repeat for multiple)
    #[arg(short = 'p', long = "package", value_name = "PACKAGE")]
    packages: Vec<String>,

    /// Toolchain name to use
    #[arg(long, hide = true)]
    toolchain_name: Option<String>,
}

impl BuildCmd {
    pub(crate) fn run(&self) -> Result<()> {
        // Construct the cargo run command
        let toolchain_name = if let Some(name) = self.toolchain_name.as_deref() {
            println!("Using toolchain_name: {name}");
            name
        } else {
            "zisk"
        };
        let mut command = Command::new("cargo");
        command.args(self.cargo_args(toolchain_name));

        // Set RUSTFLAGS for target-cpu=zisk, preserving existing flags
        if let Ok(flags) = std::env::var("RUSTFLAGS") {
            let trimmed = flags.trim();
            if !trimmed.is_empty() {
                command.env("RUSTFLAGS", trimmed);
            }
        }

        // Set up the command to inherit the parent's stdout and stderr
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        // Execute the command
        let status = command.status().context("Failed to execute cargo build command")?;
        if !status.success() {
            return Err(anyhow!("Cargo run command failed with status {}", status));
        }

        Ok(())
    }

    /// Assemble the full `cargo` argument vector for the build, including the
    /// `+<toolchain>` selector. Pure: depends only on the parsed flags, so the
    /// flag→argument wiring can be asserted without spawning a process.
    fn cargo_args(&self, toolchain_name: &str) -> Vec<String> {
        let mut args = vec![format!("+{toolchain_name}"), "build".to_string()];

        args.push("--target-dir".to_string());
        args.push(format!("target/{HELPER_TARGET_SUBDIR}"));

        if let Some(features) = &self.features {
            args.push("--features".to_string());
            args.push(features.clone());
        }
        if self.all_features {
            args.push("--all-features".to_string());
        }
        if self.no_default_features {
            args.push("--no-default-features".to_string());
        }
        if self.release {
            args.push("--release".to_string());
        }
        if let Some(artifact_dir) = &self.artifact_dir {
            args.push("--artifact-dir".to_string());
            args.push(artifact_dir.clone());
        }
        for package in &self.packages {
            args.push("--package".to_string());
            args.push(package.clone());
        }
        for bin in &self.binaries {
            args.push("--bin".to_string());
            args.push(bin.clone());
        }

        args.push("--target".to_string());
        args.push(ZISK_TARGET.to_string());

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Parse a `BuildCmd` from a bare argv (no binary name) for testing.
    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        build: BuildCmd,
    }

    fn parse(args: &[&str]) -> BuildCmd {
        let mut full = vec!["build"];
        full.extend_from_slice(args);
        Wrapper::parse_from(full).build
    }

    #[test]
    fn defaults_target_and_toolchain() {
        let args = parse(&[]).cargo_args("zisk");
        assert_eq!(args[0], "+zisk");
        assert_eq!(args[1], "build");
        assert!(args.windows(2).any(|w| w == ["--target", ZISK_TARGET]));
        assert!(args.windows(2).any(|w| w[0] == "--target-dir"));
        // No optional flags present by default.
        assert!(!args.iter().any(|a| a == "--release"));
        assert!(!args.iter().any(|a| a == "--features"));
    }

    #[test]
    fn release_and_feature_flags_wired() {
        let args =
            parse(&["--release", "--features", "a,b", "--all-features", "--no-default-features"])
                .cargo_args("custom");
        assert_eq!(args[0], "+custom");
        assert!(args.iter().any(|a| a == "--release"));
        assert!(args.windows(2).any(|w| w == ["--features", "a,b"]));
        assert!(args.iter().any(|a| a == "--all-features"));
        assert!(args.iter().any(|a| a == "--no-default-features"));
    }

    #[test]
    fn repeated_bin_and_package_flags() {
        let args = parse(&["--bin", "x", "--bin", "y", "-p", "pkg", "--artifact-dir", "out"])
            .cargo_args("zisk");
        assert_eq!(args.iter().filter(|a| *a == "--bin").count(), 2);
        assert!(args.windows(2).any(|w| w == ["--bin", "x"]));
        assert!(args.windows(2).any(|w| w == ["--bin", "y"]));
        assert!(args.windows(2).any(|w| w == ["--package", "pkg"]));
        assert!(args.windows(2).any(|w| w == ["--artifact-dir", "out"]));
    }
}
