use anyhow::Result;
use std::path::PathBuf;
use test_artifacts::ELF_BIG_INPUT;
use zisk_sdk::{ProverClient, ZiskStdin};

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

    let builder = ProverClient::embedded();
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    client.setup(&ELF_BIG_INPUT).run()?.await?;

    let result = client.execute(&ELF_BIG_INPUT, stdin.clone()).run()?.await?;

    println!(
        "ZisK has executed program with {} cycles in {} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    println!("Generating proof...");
    client.prove(&ELF_BIG_INPUT, stdin.clone()).run()?.await?;

    println!("\u{2713} Prove completed successfully!");

    Ok(())
}
