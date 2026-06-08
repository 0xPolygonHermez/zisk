use anyhow::Result;
use sha2::{Digest, Sha256};
use sha_hasher_common::Output;
use sha_hasher_host::ELF_SHA_HASHER;
use zisk_sdk::{ExecutorKind, Proof, ProverClient, PublicValues, ZiskStdin};

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
    let coordinator_url =
        std::env::var("ZISK_COORDINATOR_URL").unwrap_or_else(|_| "http://127.0.0.1:15100".into());
    let client = ProverClient::remote(coordinator_url).build()?;

    println!("Setting up program...");
    client.upload(&ELF_SHA_HASHER).run()?;
    client.setup(&ELF_SHA_HASHER).run()?.await?;
    println!("Setup completed successfully");

    println!("Generating proof (this may take a while)...");
    let result =
        client.prove(&ELF_SHA_HASHER, stdin).executor(ExecutorKind::Emulator).run()?.await?;
    println!("Proof generated successfully in {} ms", result.get_proving_time());
    println!("Execution steps: {}", result.get_execution_steps());

    println!("Verifying proof...");
    result.verify()?;
    println!("Proof verification successful!");

    println!("Saving proof to disk...");
    let proof_path = std::path::Path::new("tmp/sha_hasher_proof.bin");
    if let Some(parent) = proof_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    result.save_proof(proof_path)?;
    println!("Proofs saved to tmp/ directory");

    let mut hash = [0u8; 32];
    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }

    let output = Output { hash: hash.into(), iterations: n, magic_number: 0xDEADBEEF };
    println!("Expected output hash: {:02x?}", &hash[..8]);

    println!("Verifying saved proofs from disk...");
    let publics = PublicValues::write_abi(&output)?;
    let vk = ELF_SHA_HASHER.vk()?;

    println!("Loading proof with publics from disk...");
    let proof = Proof::load(proof_path)?;

    println!("Verifying proof with embedded publics...");
    // Verify the proof with its embedded publics (from guest's commit)
    proof.with_program_vk(&vk).with_publics(&publics).verify()?;
    println!("Proof verification successful!");

    println!("\u{2713} Successfully generated and verified all proofs!");

    Ok(())
}
