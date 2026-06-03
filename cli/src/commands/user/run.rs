use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};
use zisk_build::{HELPER_TARGET_SUBDIR, ZISK_TARGET, ZISK_VERSION_MESSAGE};
use zisk_common::io::ZiskStdin;
use zisk_prover_backend::{GuestProgram, ProfilingMode};

use crate::common::detect_current_project_elf;

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

    /// Build artifacts in release mode, with optimizations
    #[arg(long)]
    release: bool,

    /// Do not activate the `default` feature
    #[arg(long)]
    no_default_features: bool,

    /// Path to the guest ELF
    #[arg(short = 'e', long)]
    elf: Option<String>,

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
                command.args(["+zisk", "build"]);
                command.args(["--target-dir", &format!("target/{}", HELPER_TARGET_SUBDIR)]);
                if let Some(features) = &self.features {
                    command.arg("--features").arg(features);
                }
                if self.all_features {
                    command.arg("--all-features");
                }
                if self.no_default_features {
                    command.arg("--no-default-features");
                }
                if self.release {
                    command.arg("--release");
                }
                command.args(["--target", ZISK_TARGET]);
                command.stdout(Stdio::inherit());
                command.stderr(Stdio::inherit());

                let status = command.status().context("Failed to execute cargo build command")?;
                if !status.success() {
                    return Err(anyhow!("cargo build command failed with status {}", status));
                }

                detect_current_project_elf()?
                    .ok_or_else(|| anyhow!("Could not find built ELF. Make sure you are in a Cargo project directory."))?
                    .to_string_lossy()
                    .into_owned()
            }
        };

        let program = GuestProgram::from_uri(&elf_path)?;
        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;
        program.run_emulation(stdin, self.profiling)
    }
}
