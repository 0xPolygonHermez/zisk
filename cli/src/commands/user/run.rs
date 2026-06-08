use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};
use zisk_build::{HELPER_TARGET_SUBDIR, ZISK_TARGET, ZISK_VERSION_MESSAGE};
use zisk_common::io::ZiskStdin;
use zisk_prover_backend::{GuestProgram, ProfilingMode};

use crate::common::{detect_project_elf_for_profile, ElfSelectorArgs, Profile};

// Structure representing the 'run' subcommand of cargo-zisk
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Build the program and run it using the ZisK toolchain
pub(crate) struct RunCmd {
    /// Space or comma separated list of features to activate
    #[arg(short = 'F', long)]
    features: Option<String>,

    /// Activate all available features
    #[arg(long)]
    all_features: bool,

    /// Do not activate the `default` feature
    #[arg(long)]
    no_default_features: bool,

    /// Path to the guest ELF
    #[arg(short = 'e', long)]
    elf: Option<String>,

    #[command(flatten)]
    selector: ElfSelectorArgs,

    /// Input for the guest. Accepts a file path, `file://path`, or inline data
    /// `inline://[[1,2],[3]]` (a JSON array of u64 arrays, one frame per inner array)
    #[arg(short = 'i', long)]
    inputs: Option<String>,

    /// Profiling report to emit
    #[arg(short = 'p', long)]
    profiling: Option<ProfilingMode>,
}

// Implement the run functionality for ZiskRun
impl RunCmd {
    pub(crate) fn run(&self) -> Result<()> {
        let elf_path = match &self.elf {
            Some(path) => path.clone(),
            None => {
                // Build first, then detect the resulting ELF
                let mut command = Command::new("cargo");
                command.args(self.cargo_build_args());
                command.stdout(Stdio::inherit());
                command.stderr(Stdio::inherit());

                let status = command.status().context("Failed to execute cargo build command")?;
                if !status.success() {
                    return Err(anyhow!("cargo build command failed with status {}", status));
                }

                // Detect the ELF for the profile we just built (and the selected
                // binary, if any) rather than guessing release-then-debug.
                detect_project_elf_for_profile(self.selector.profile(), self.selector.bin())?
                    .ok_or_else(|| anyhow!("Could not find built ELF. Make sure you are in a Cargo project directory."))?
                    .to_string_lossy()
                    .into_owned()
            }
        };

        let program = GuestProgram::from_uri(&elf_path)?;
        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;
        program.run_emulation(stdin, self.profiling)
    }

    /// Assemble the `cargo` argument vector used to build the guest before
    /// running it. Pure: depends only on the parsed flags.
    fn cargo_build_args(&self) -> Vec<String> {
        let mut args = vec!["+zisk".to_string(), "build".to_string()];
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
        if self.selector.profile() == Profile::Release {
            args.push("--release".to_string());
        }
        if let Some(bin) = self.selector.bin() {
            args.push("--bin".to_string());
            args.push(bin.to_string());
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

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        run: RunCmd,
    }

    fn parse(args: &[&str]) -> RunCmd {
        let mut full = vec!["run"];
        full.extend_from_slice(args);
        Wrapper::parse_from(full).run
    }

    #[test]
    fn build_args_defaults() {
        let args = parse(&[]).cargo_build_args();
        assert_eq!(&args[0..2], &["+zisk", "build"]);
        assert!(args.windows(2).any(|w| w == ["--target", ZISK_TARGET]));
        assert!(args.windows(2).any(|w| w[0] == "--target-dir"));
        assert!(!args.iter().any(|a| a == "--release"));
    }

    #[test]
    fn build_args_with_flags() {
        let args =
            parse(&["--release", "--features", "x", "--no-default-features"]).cargo_build_args();
        assert!(args.iter().any(|a| a == "--release"));
        assert!(args.windows(2).any(|w| w == ["--features", "x"]));
        assert!(args.iter().any(|a| a == "--no-default-features"));
    }

    #[test]
    fn build_args_with_bin() {
        let args = parse(&["--bin", "execute"]).cargo_build_args();
        assert!(args.windows(2).any(|w| w == ["--bin", "execute"]));
        // Absent by default.
        assert!(!parse(&[]).cargo_build_args().iter().any(|a| a == "--bin"));
    }
}
