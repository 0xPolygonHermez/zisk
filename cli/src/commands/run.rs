use crate::{ZISK_TARGET, ZISK_VERSION_MESSAGE};
use anyhow::{anyhow, Context, Result};
use std::{
    env,
    process::{Command, Stdio},
};

use std::{fs::File, io::Write, path::Path};

// Structure representing the 'run' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
pub struct ZiskRun {
    #[clap(short = 'F', long)]
    features: Option<String>,

    #[clap(long)]
    all_features: bool,

    #[clap(long)]
    release: bool,

    #[clap(long)]
    no_default_features: bool,

    #[clap(long, short)]
    qemu: bool,

    #[clap(short = 'x', long)]
    stats: bool,

    #[clap(long)]
    gdb: bool,

    #[clap(short = 'i', long)]
    input: Option<String>,

    #[clap(long, short = 'm')]
    metrics: bool,

    #[clap(last = true)]
    args: Vec<String>,
}

// Implement the run functionality for ZiskRun
impl ZiskRun {
    pub fn run(&self) -> Result<()> {
        let runner_command: String;
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
        if !self.qemu {
            let mut extra_command: String = "".to_string();
            let mut input_command: String = "".to_string();
            if self.stats {
                extra_command += " -x ";
            }
            if self.metrics {
                extra_command += " -m ";
            }
            if self.input.is_some() {
                let path = Path::new(self.input.as_ref().unwrap());
                if !path.exists() {
                    return Err(anyhow!("Input file does not exist at path: {}", path.display()));
                }
                input_command = format!("-i {}", self.input.as_ref().unwrap());
            }
            runner_command = format!("ziskemu {} {} -e", input_command, extra_command);
        } else {
            let mut gdb_command = "";
            if self.gdb {
                gdb_command = "-S";
            }

            let input_path: &Path = Path::new(self.input.as_ref().unwrap());

            if !input_path.exists() {
                return Err(anyhow!("Input file does not exist at path: {}", input_path.display()));
            }

            let build_path = match input_path.parent() {
                Some(parent) => parent.to_str().unwrap_or("./"),
                None => "./",
            };

            let stem = input_path.file_stem().unwrap_or_default();
            let extension = input_path.extension().unwrap_or_default();
            let output_path = format!(
                "{}/{}_size.{}",
                build_path,
                stem.to_str().unwrap_or(""),
                extension.to_str().unwrap_or("")
            );

            let metadata = std::fs::metadata(input_path)?;
            let file_size = metadata.len();

            let size_bytes = file_size.to_le_bytes();
            let mut output_file = File::create(output_path.clone())?;
            output_file.write_all(&size_bytes)?;

            runner_command = format!(
                "
            qemu-system-riscv64 \
            -cpu rv64 \
            -machine virt \
            -device loader,file=./{},addr=0x90000000 \
            -device loader,file=./{},addr=0x90000008 \
            -m 1G \
            -s \
            {}  \
            -nographic \
            -serial mon:stdio \
            -bios none \
            -kernel",
                output_path,
                input_path.display(),
                gdb_command
            );
        }

        env::set_var("CARGO_TARGET_RISCV64IMA_POLYGON_ZISKOS_ELF_RUNNER", runner_command);

        command.args(["--target", ZISK_TARGET]);

        // Add any additional arguments passed to the run command
        command.args(&self.args);

        // Set up the command to inherit the parent's stdout and stderr
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        // Execute the command
        let status = command.status().context("Failed to execute cargo run command")?;
        if !status.success() {
            return Err(anyhow!("Cargo run command failed with status {}", status));
        }

        Ok(())
    }
}
