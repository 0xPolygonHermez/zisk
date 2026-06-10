use anyhow::Result;
use sha_hasher_host::ELF_SHA_HASHER;
use zisk_sdk::{ProofKind, ProverClient, ZiskStdin};

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
    let coordinator_url =
        std::env::var("ZISK_COORDINATOR_URL").unwrap_or_else(|_| "http://127.0.0.1:15100".into());
    let client = ProverClient::remote(coordinator_url).build()?;

    println!("Setting up program...");
    client.upload(&ELF_SHA_HASHER).run()?;
    client.setup(&ELF_SHA_HASHER).run()?.await?;
    println!("Setup completed successfully");

    println!("Generating Vadcop proof...");
    let vadcop_result = client.prove(&ELF_SHA_HASHER, stdin).run()?.await?;
    println!("Vadcop proof generated in {} ms", vadcop_result.get_proving_time());

    println!("Reducing proof (this may take a while)...");
    let result =
        client.wrap_proof(vadcop_result.get_proof(), ProofKind::VadcopFinalMinimal).run()?.await?;

    println!("Verifying minimal proof...");
    result.verify()?;
    println!("Minimal proof verification successful!");

    println!("\u{2713} Successfully generated and verified minimal proof!");

    Ok(())
}
