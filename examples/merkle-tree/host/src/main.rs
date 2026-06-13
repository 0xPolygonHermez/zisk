//! ZisK host: loads the guest ELF, writes the leaf count, and runs the
//! emulator with summary profiling enabled.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -- <n>
//! ```
//!
//! Where `<n>` is the number of leaves as a u32. Defaults to `1000` when no
//! argument or wrong argument is provided.

use std::error::Error;

use zisk_sdk::{GuestProgram, ProfilingMode, ZiskStdin, load_program};

/// Guest ELF binary, embedded into the host at build time.
static PROGRAM_INLINE: GuestProgram = load_program!("inline-guest");
static PROGRAM_BASE: GuestProgram = load_program!("merkle-guest");

fn main() -> Result<(), Box<dyn Error>> {
    // Number of leaves to hash into the Merkle tree. This example profiles the
    // emulator rather than proving, so the `--asm`/`--gpu` prover flags do not
    // apply here.
    let (n, _, _) = examples_utils::parse_args(1000u32);

    // Write the leaf count into the guest's standard input stream.
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    let n: u32 = stdin.read()?;
    println!("Input prepared: {} iterations", n);

    // Run the guest inside the ZisK emulator with complete profiling output.
    zisk_sdk::run(&PROGRAM_INLINE, stdin.clone(), Some(ProfilingMode::Inline))?;
    zisk_sdk::run(&PROGRAM_BASE, stdin, Some(ProfilingMode::Summary))?;

    Ok(())
}
