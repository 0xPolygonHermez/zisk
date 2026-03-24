use anyhow::Result;
use std::path::PathBuf;
use zisk_sdk::{include_guest_elf, EmbeddedGuestElf, GuestProgram, ProverClient, ZiskStdin};

pub const ELF: EmbeddedGuestElf = include_guest_elf!("big-program-guest");

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

    let (pk, _vkey) = client.setup(&GuestProgram::from_elf(ELF))?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(&pk, stdin.clone())?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.executor_summary.steps, result.total_duration
    );

    println!("Generating proof...");
    client.prove(&pk, stdin.clone()).run()?;

    println!("\u{2713} Prove completed successfully!");

    Ok(())
}
