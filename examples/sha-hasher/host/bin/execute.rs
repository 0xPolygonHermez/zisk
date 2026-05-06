use anyhow::Result;
use examples_common::{build_client, ClientConfig};
use sha_hasher_common::Output;
use zisk_sdk::{load_program, GuestProgram, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    println!("Building prover client...");
    let client = build_client(ClientConfig::default())?;

    println!("Setting up program...");
    client.upload(&PROGRAM).run()?;
    client.setup(&PROGRAM).run()?.await?;
    println!("Setup completed successfully");

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing program (no proof generation)...");
    let result = client.execute(&PROGRAM, stdin.clone()).run()?.await?;

    println!("\u{2713} Execution completed successfully!");
    println!("Cycles: {}", result.get_execution_steps());
    println!("Duration: {} ms", result.get_execution_time());

    println!("Reading public outputs...");
    let output: Output = result.get_public_values_abi()?;
    println!("Public outputs:");
    println!("  Hash: {:02x?}", output.hash);
    println!("  Iterations: {}", output.iterations);
    println!("  Magic number: 0x{:08x}", output.magic_number);

    Ok(())
}
