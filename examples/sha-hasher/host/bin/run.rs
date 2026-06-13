use sha_hasher_host::ELF_SHA_HASHER;
use std::error::Error;
use zisk_sdk::{ProfilingMode, ZiskStdin};

fn main() -> Result<(), Box<dyn Error>> {
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    let n: u32 = stdin.read()?;
    println!("Input prepared: {} iterations", n);

    println!("Running ZisK Emulator...");
    zisk_sdk::run(&ELF_SHA_HASHER, stdin, Some(ProfilingMode::Complete))?;
    println!("ZisK Emulator completed successfully!");

    Ok(())
}
