use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zisk_sdk::{ProverClient, elf_path, ZiskStdin, ZiskIO, ElfBinaryFromFile};

#[derive(Serialize, Deserialize, Debug)]
struct Output {
    hash: [u8; 32],
    iterations: u32,
    magic_number: u32,
}

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    let elf = ElfBinaryFromFile::new(&PathBuf::from(elf_path!("sha-hasher-guest")), false)?;

    let current_dir = std::env::current_dir()?;
    let stdin =
        ZiskStdin::from_file(current_dir.join("sha-hasher/host/tmp/verify_constraints_input.bin"))?;

    let n: u32 = stdin.read()?;
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client...");
    let client = ProverClient::builder().emu().verify_constraints().build().unwrap();

    println!("Setting up program...");
    client.setup(&elf)?;
    println!("Setup completed successfully");

    println!("Verifying constraints (no proof generation)...");
    let result = client.verify_constraints(stdin.clone())?;

    println!("\u{2713} VerifyConstraints completed successfully!");
    println!("Cycles: {}", result.get_execution_steps());
    println!("Duration: {:?}", result.get_duration());

    println!("Reading public outputs...");
    let output: Output = result.get_public_values()?;
    println!("Public outputs:");
    println!("  Hash: {:02x?}", output.hash);
    println!("  Iterations: {}", output.iterations);
    println!("  Magic number: 0x{:08x}", output.magic_number);

    Ok(())
}
