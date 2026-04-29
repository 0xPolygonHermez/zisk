use anyhow::Result;
use std::path::PathBuf;
use zisk_sdk::{load_program, GuestProgram, ProverClient, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("big-program-guest");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Read the input size that was configured during build
    let size_mb: usize = env!("INPUT_SIZE_MB").parse().unwrap();

    // Use CARGO_MANIFEST_DIR to get absolute path to the crate directory
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let input_path =
        PathBuf::from(manifest_dir).join(format!("tmp/big_program_input_{}mb.bin", size_mb));
    println!("Loading input from: {} ({}MB)", input_path.display(), size_mb);

    let stdin = ZiskStdin::from_file(&input_path)?;
    println!("Input loaded successfully");

    // Create a `ProverClient` method.
    let client = ProverClient::embedded().build()?;

    client.setup(&PROGRAM).run()?.await?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(&PROGRAM, stdin.clone()).run()?.await?;

    println!(
        "ZisK has executed program with {} cycles in {:?} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    println!("Generating proof...");
    client.prove(&PROGRAM, stdin.clone()).run()?.await?;

    println!("\u{2713} Prove completed successfully!");

    Ok(())
}
