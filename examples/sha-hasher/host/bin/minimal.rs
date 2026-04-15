use anyhow::Result;
use zisk_sdk::{load_program, GuestProgram, ProofKind, ProverClient, ProverOpts, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

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
    let proof_opts = ProverOpts::default().minimal_memory();
    let client = ProverClient::embedded().with_prover_options(proof_opts).gpu().build()?;

    println!("Setting up program...");
    client.setup(&PROGRAM).run()?.await?;
    println!("Setup completed successfully");

    println!("Generating Vadcop proof...");
    let vadcop_result = client.prove(&PROGRAM, stdin).run()?.await?;
    println!("Vadcop proof generated in {:?}", vadcop_result.get_duration());

    println!("Reducing proof (this may take a while)...");
    let result =
        client.wrap_proof(vadcop_result.get_proof(), ProofKind::VadcopFinalMinimal).run()?.await?;

    // Alternatively, you can also call `minimal()` on the `ProverClient.prove` method to generate a minimal proof directly.
    // let result = client.prove(&PROGRAM, stdin)?.with_prover_options(proof_opts).minimal().run()?;

    println!("Verifying minimal proof...");
    result.verify()?;
    println!("Minimal proof verification successful!");

    println!("\u{2713} Successfully generated and verified minimal proof!");

    Ok(())
}
