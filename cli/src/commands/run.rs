use anyhow::{anyhow, Context, Result};
use std::{
    env,
    process::{Command, Stdio},
};
use zisk_build::{ZISK_TARGET, ZISK_VERSION_MESSAGE};

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ProfilingMode {
    Inline,
    Summary,
    Complete,
}

// Structure representing the 'run' subcommand of cargo-zisk
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Build the program and run it using the ZisK toolchain
pub struct ZiskRun {
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

    /// Input file path for the guest. Accepts a string literal or a path to a binary file
    #[arg(short = 'i', long)]
    inputs: Option<String>,

    /// Profiling report to emit
    #[arg(short = 'p', long)]
    profiling: Option<ProfilingMode>,

    // Hidden flags
    /// Log the output to console in riscof format
    #[arg(short = 'f', long, hide = true)]
    log_output_riscof: bool,

    /// Additional arguments to pass to the cargo run command
    #[arg(last = true, hide = true)]
    args: Vec<String>,
}

// Implement the run functionality for ZiskRun
impl ZiskRun {
    pub fn run(&self) -> Result<()> {
        match &self.elf {
            Some(_) => self.run_cmd(Command::new(self.build_ziskemu_cmd())),
            None => self.cargo_run_cmd(),
        }
    }

    fn run_cmd(&self, mut command: Command) -> Result<()> {
        // Set up the command to inherit the parent's stdout and stderr
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        // Execute the ziskemu command
        let status = command.status().context("Failed to execute ziskemu command")?;
        if !status.success() {
            return Err(anyhow!("ziskemu command failed with status {}", status));
        }

        Ok(())
    }

    fn cargo_run_cmd(&self) -> Result<()> {
        // Construct the cargo run command
        let mut command = Command::new("cargo");
        command.args(["+zisk", "run"]);

        // Add the feature selection flags
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

        env::set_var("CARGO_TARGET_RISCV64IMA_ZISK_ZKVM_ELF_RUNNER", self.build_ziskemu_cmd());

        command.args(["--target", ZISK_TARGET]);

        // Add any additional arguments passed to the run command
        command.args(&self.args);

        self.run_cmd(command)
    }

    fn build_ziskemu_cmd(&self) -> String {
        let mut cmd = "ziskemu".to_string();
        if let Some(elf) = &self.elf {
            cmd += &format!(" -e {}", elf);
        }
        if let Some(input) = &self.inputs {
            cmd += &format!(" -i {}", input);
        }
        if self.log_output_riscof {
            cmd += " -f";
        }
        if self.profiling.is_some() {
            cmd += " -p ";
            cmd += match self.profiling.unwrap() {
                ProfilingMode::Inline => "—sdk —profile-tags",
                ProfilingMode::Summary => "—sdk —opcodes —top-functions",
                ProfilingMode::Complete => "—sdk —profiler-output",
            };
        }
        cmd
    }
}
