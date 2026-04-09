use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zisk_sdk::{
    load_program, ExecutorKind, GuestProgram, ProofOpts, ProverClient, ZiskProofWithPublicValues,
    ZiskPublics, ZiskStdin,
};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

#[derive(Serialize, Deserialize, Debug)]
struct Output {
    hash: [u8; 32],
    iterations: u32,
    magic_number: u32,
}

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client...");
    let client = ProverClient::embedded().gpu().assembly().build()?;

    println!("Setting up program...");
    client.setup(&PROGRAM).run()?;
    println!("Setup completed successfully");

    println!("Generating proof (this may take a while)...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let result = client
        .prove(&PROGRAM, stdin)
        .executor(ExecutorKind::Assembly)
        .with_proof_options(proof_opts)
        .run()?;
    println!("Proof generated successfully in {:?}", result.get_duration());
    println!("Execution steps: {}", result.get_execution_steps());

    println!("Verifying proof...");
    result.verify()?;
    println!("Proof verification successful!");

    println!("Saving proof to disk...");
    result.save_proof("tmp/sha_hasher_proof.bin")?;
    println!("Proofs saved to tmp/ directory");

    let mut hash = [0u8; 32];
    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }

    let output = Output { hash, iterations: n, magic_number: 0xDEADBEEF };
    println!("Expected output hash: {:02x?}", &hash[..8]);

    println!("Verifying saved proofs from disk...");
    let publics = ZiskPublics::write(&output)?;
    println!("Loading proof from disk...");
    let vk = client.vk(&PROGRAM)?;

    println!("Loading proof with publics from disk...");
    let proof_with_publics = ZiskProofWithPublicValues::load("tmp/sha_hasher_proof.bin")?;
    println!("Verifying proof with publics...");
    proof_with_publics.with_program_vk(&vk).with_publics(&publics).verify()?;
    println!("Proof with publics verification successful!");

    println!("\u{2713} Successfully generated and verified all proofs!");

    Ok(())
}
