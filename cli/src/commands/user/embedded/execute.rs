use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{EmbeddedClientBuilder, ExecuteOutput, GuestProgram, ZiskHints, ZiskStdin};

use super::validate_asm_hints;
use crate::common::resolve_elf;
use crate::ux::print_job_banner;

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Execute a guest program locally
pub(crate) struct ZiskEmbeddedExecute {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,

    /// Input for the guest. Accepts a file path, `file://path`, or inline data
    /// `inline://[[1,2],[3]]` (a JSON array of u64 arrays, one frame per inner array)
    #[arg(short = 'i', long)]
    inputs: Option<String>,

    /// Precompiles hints URI for the guest. Requires the ASM backend (`--asm`).
    ///
    /// # URI Formats
    /// - `None` → null stream (no input)
    /// - `"scheme://resource"` → parsed based on scheme
    /// - No scheme → treated as a file path
    ///
    /// # Supported Schemes
    /// - `file://path/to/file`   → File-based stream
    /// - `unix://path/to/socket` → Unix domain socket stream
    #[arg(long, conflicts_with = "inputs")]
    hints: Option<String>,

    /// Use the ASM emulator instead of the default Rust emulator
    #[arg(short = 'a', long)]
    asm: bool,

    /// Verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskEmbeddedExecute {
    pub(crate) fn run(&mut self) -> Result<()> {
        let elf = resolve_elf(self.elf.take())?;
        validate_asm_hints(self.asm, self.hints.as_deref())?;

        print_job_banner(
            &format!("{} Execute", "EMBEDDED".bold()),
            &elf,
            self.inputs.as_deref(),
            self.hints.as_deref(),
        );
        println!();

        let program = GuestProgram::from_uri(elf.to_str().unwrap())?;
        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;
        let hints = self.hints.as_ref().map(ZiskHints::from_uri).transpose()?;

        let mut builder = EmbeddedClientBuilder::default().verbose(self.verbose);
        if self.asm {
            builder = builder.assembly();
        }
        let client = builder.execute_only().build()?;

        client.setup(&program, hints.is_some())?;

        let request = client.execute(&program, stdin, hints)?;

        info!("{}", "--- EXECUTE SUMMARY -----------".bright_green().bold());
        print_executeoutput(&request);

        Ok(())
    }
}

fn print_executeoutput(output: &ExecuteOutput) {
    print_execution_summary(output);
    print_execution_breakdown(output);
    print_plan_summary(output);
}

fn print_execution_summary(output: &ExecuteOutput) {
    let steps = output.get_execution_steps();
    let time = output.get_execution_time();
    let cost =
        output.get_execution_cost().map(|c| format!("{} cells", c)).unwrap_or("N/A".to_string());
    info!("Execution completed in {}ms, steps: {}, cost: {}", time, steps, cost);
}

fn print_execution_breakdown(output: &ExecuteOutput) {
    let et = output.get_executor_time();
    info!(
        "Execution time breakdown: {}ms (execution: {}ms + plan: {}ms + plan mem: {}ms)",
        et.total_duration,
        et.execution_duration,
        et.count_and_plan_duration,
        et.count_and_plan_mo_duration,
    );

    if let Some(aei) = &et.asm_execution_duration {
        info!("Assembly execution speed: {:.3}ms ({:.0} MHz)", aei.time * 1000f32, aei.mhz);
    }
}

fn print_plan_summary(output: &ExecuteOutput) {
    if let Some(plan) = output.get_plan() {
        info!("Plan:");
        for entry in plan {
            info!("  {}: {}", entry.name, entry.count);
        }
    }
}
