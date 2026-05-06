use anyhow::Result;
use examples_common::{build_client, ClientConfig};
use zisk_sdk::{load_program, GuestProgram, ProofKind, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client (Minimal proof mode)...");

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

    println!("Generating minimal proof (this may take a while)...");
    let result = client.prove(&PROGRAM, stdin).wrap(ProofKind::VadcopFinalMinimal).run()?.await?;
    println!("Minimal proof generated in {} ms", result.get_proving_time());

    println!("Verifying minimal proof...");
    result.verify()?;
    println!("Minimal proof verification successful!");

    println!("\u{2713} Successfully generated and verified minimal proof!");

    Ok(())
}
