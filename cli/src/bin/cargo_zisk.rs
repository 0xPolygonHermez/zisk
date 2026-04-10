use anyhow::{Context, Result};
use std::env;
use std::process::{Command, exit};

/// Main entry point for cargo-zisk binary.
///
/// This binary acts as a proxy: when the --gpu flag is present on one of the
/// GPU-capable commands (prove, verify-constraints, check-setup), it delegates
/// execution to cargo-zisk-gpu; otherwise it delegates to cargo-zisk-cpu.
/// Both binaries must be in the same directory.
fn main() -> Result<()> {
    // Collect all arguments (skip program name)
    let args: Vec<String> = env::args().skip(1).collect();

    // Commands that support the --gpu flag
    const GPU_COMMANDS: &[&str] =
        &["prove", "verify-constraints", "check-setup", "rom-setup", "prove-snark"];

    // Find the first subcommand (first non-flag argument)
    let first_cmd = args.iter().find(|a| !a.starts_with('-')).map(|s| s.as_str());

    // Use GPU binary only when the command supports --gpu AND --gpu is present
    let is_gpu_command = first_cmd.map_or(false, |c| GPU_COMMANDS.contains(&c));
    let has_gpu_flag = args.iter().any(|arg| arg == "--gpu");
    let use_gpu = is_gpu_command && has_gpu_flag;

    let bin_name = if use_gpu { "cargo-zisk-gpu" } else { "cargo-zisk-cpu" };
    let target_args = args;

    // Resolve the binary path relative to the current executable
    let current_exe = env::current_exe().context("Failed to get current cargo-zisk path")?;
    let exe_dir = current_exe.parent().context("Failed to get cargo-zisk directory")?;
    let target_binary = exe_dir.join(bin_name);

    if !target_binary.exists() {
        eprintln!("Error: {} binary not found at {:?}", bin_name, target_binary);
        exit(1);
    }

    let status = Command::new(&target_binary)
        .args(&target_args)
        .status()
        .with_context(|| format!("Failed to execute {:?}", target_binary))?;

    exit(status.code().unwrap_or(1));
}
