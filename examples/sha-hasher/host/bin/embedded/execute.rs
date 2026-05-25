use anyhow::Result;
use sha_hasher_common::Output;
use sha_hasher_host::ELF_SHA_HASHER;
use zisk_sdk::{ExecutorKind, ProverClient, ZiskStdin};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client...");
    let builder = ProverClient::embedded().assembly();
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    println!("Setting up program...");
    client.setup(&ELF_SHA_HASHER).run()?.await?;
    println!("Setup completed successfully");

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing program (no proof generation)...");
    let result = client
        .execute(&ELF_SHA_HASHER, stdin.clone())
        .executor(ExecutorKind::Emulator)
        .run()?
        .await?;
    println!("\u{2713} Execution completed successfully!");
    println!("Cycles: {}", result.get_execution_steps());
    println!("Duration: {} ms", result.get_execution_time());

    let result = client
        .execute(&ELF_SHA_HASHER, stdin.clone())
        .executor(ExecutorKind::Assembly)
        .run()?
        .await?;

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
