use anyhow::Result;
use zisk_sdk::{load_program, GuestProgram, Proof, ProofKind, ProverClient, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client (SNARK mode)...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client with SNARK support...");
    let builder = ProverClient::embedded().plonk();
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    println!("Setting up program and generating verification key...");
    client.setup(&PROGRAM).run()?.await?;
    println!("Setup completed successfully");

    println!("Generating PLONK proof (this may take a while)...");
    let snark_proof = client.prove(&PROGRAM, stdin).wrap(ProofKind::Plonk).run()?.await?;
    println!("PLONK proof generated successfully in {} ms", snark_proof.get_proving_time());
    println!("Execution steps: {}", snark_proof.get_execution_steps());

    println!("Verifying PLONK proof...");
    snark_proof.verify()?;
    println!("PLONK proof verification successful!");

    println!("Saving PLONK proof to disk...");
    snark_proof.save_proof("/tmp/sha_hasher_proof_snark.bin")?;
    println!("Proof saved to /tmp/sha_hasher_proof_snark.bin");

    println!("Loading and verifying saved PLONK proof...");
    let proof = Proof::load("/tmp/sha_hasher_proof_snark.bin")?;
    let vkey = PROGRAM.vk()?;
    proof.with_program_vk(&vkey).verify()?;
    println!("Saved PLONK proof verification successful!");

    println!("\u{2713} Successfully generated and verified PLONK proof!");

    Ok(())
}
