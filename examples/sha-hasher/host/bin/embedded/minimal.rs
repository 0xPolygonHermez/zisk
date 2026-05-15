use anyhow::Result;
use test_artifacts::ELF_SHA_HASHER;
use zisk_sdk::{ExecutorKind, ProofKind, ProverClient, ZiskStdin};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client (Minimal proof mode)...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client...");
    let builder = ProverClient::embedded().executor(ExecutorKind::Assembly);
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    println!("Setting up program...");
    client.setup(&ELF_SHA_HASHER).run()?.await?;
    println!("Setup completed successfully");

    println!("Generating minimal proof (this may take a while)...");
    let result = client
        .prove(&ELF_SHA_HASHER, stdin)
        .executor(ExecutorKind::Assembly)
        .wrap(ProofKind::VadcopFinalMinimal)
        .run()?
        .await?;
    println!("Minimal proof generated in {} ms", result.get_proving_time());

    println!("Verifying minimal proof...");
    result.verify()?;
    println!("Minimal proof verification successful!");

    println!("\u{2713} Successfully generated and verified minimal proof!");

    Ok(())
}
