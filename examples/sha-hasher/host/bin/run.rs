use anyhow::Result;
use zisk_sdk::{load_program, GuestProgram, ProfilingMode, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

fn main() -> Result<()> {
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    let n: u32 = stdin.read()?;
    println!("Input prepared: {} iterations", n);

    println!("Running ZisK Emulator...");
    zisk_sdk::run(&PROGRAM, stdin, Some(ProfilingMode::Complete))?;
    println!("ZisK Emulator completed successfully!");

    Ok(())
}
