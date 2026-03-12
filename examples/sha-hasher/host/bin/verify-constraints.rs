use anyhow::Result;
use serde::{Deserialize, Serialize};
use zisk_sdk::{include_elf, ElfBinary, ProverClient, ZiskStdin};

pub const ELF: ElfBinary = include_elf!("sha-hasher-guest");

#[derive(Serialize, Deserialize, Debug)]
struct Output {
    hash: [u8; 32],
    iterations: u32,
    magic_number: u32,
}

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    let current_dir = std::env::current_dir()?;
    let stdin =
        ZiskStdin::from_file(current_dir.join("sha-hasher/host/tmp/verify_constraints_input.bin"))?;

    let n: u32 = stdin.read()?;
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client...");
    let client = ProverClient::builder().emu().verify_constraints().build().unwrap();

    println!("Setting up program...");
    let (pk, _vkey) = client.setup(&ELF)?;
    println!("Setup completed successfully");

    println!("Verifying constraints (no proof generation)...");
    let result = client.verify_constraints(&pk, stdin.clone())?;

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
