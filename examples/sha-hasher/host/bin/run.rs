use anyhow::Result;
use zisk_sdk::{load_program, EmuOptions, GuestProgram, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

fn main() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let stdin =
        ZiskStdin::from_file(current_dir.join("sha-hasher/host/tmp/verify_constraints_input.bin"))?;

    let n: u32 = stdin.read()?;
    println!("Input prepared: {} iterations", n);

    println!("Running ZisK Emulator...");
    let emu_options = EmuOptions {
        stats: true,
        read_symbols: true,
        top_roi_detail: true,
        ..EmuOptions::default()
    };
    PROGRAM.run(stdin.into_inner(), &emu_options)?;
    println!("ZisK Emulator completed successfully!");

    Ok(())
}
