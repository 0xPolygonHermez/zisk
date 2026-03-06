use anyhow::Result;
use std::path::PathBuf;
use zisk_sdk::{include_elf, ElfBinary, ProofOpts, ProverClient, ZiskIO, ZiskStdin};

pub const ELF: ElfBinary = include_elf!("big-program-guest");

fn main() -> Result<()> {
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
    let client = ProverClient::builder()
        .asm()
        .verify_constraints()
        .proving_key_path_opt(Some("/home/roger/zisk/build/provingKey".into()))
        .build()
        .unwrap();

    let (pk, vkey) = client.setup(&ELF)?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(&pk, stdin.clone())?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.executor_summary.steps, result.total_duration
    );

    println!("Verifying constraints (no proof generation)...");
    client.verify_constraints(&pk, stdin.clone())?;

    println!("\u{2713} VerifyConstraints completed successfully!");

    Ok(())
}
