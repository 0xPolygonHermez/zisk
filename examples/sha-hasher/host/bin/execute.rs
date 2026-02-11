use anyhow::Result;
use serde::{Deserialize, Serialize};
use zisk_sdk::{ZiskStdin, ZiskIO, ElfBinary, ProverClient, include_elf};

pub const ELF: ElfBinary = include_elf!("sha-hasher-guest");

#[derive(Serialize, Deserialize, Debug)]
struct Output {
    hash: [u8; 32],
    iterations: u32,
    magic_number: u32,
}

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client...");
    let client = ProverClient::builder().asm().base_port(54321).build().unwrap();

    println!("Setting up program...");
    client.setup(&ELF)?;
    println!("Setup completed successfully");

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing program (no proof generation)...");
    let result = client.execute(stdin.clone())?;

    println!("\u{2713} Execution completed successfully!");
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
