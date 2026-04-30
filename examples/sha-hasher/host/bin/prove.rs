use anyhow::Result;
use sha2::{Digest, Sha256};
use sha_hasher_common::Output;
use zisk_sdk::{
    load_program, ExecutorKind, GuestProgram, Proof, ProverClient, PublicValues, ZiskStdin,
};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

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
    let client = ProverClient::remote("http://127.0.0.1:7000").build()?;

    println!("Setting up program...");
    client.upload(&PROGRAM).run()?;
    client.setup(&PROGRAM).run()?.await?;
    println!("Setup completed successfully");

    println!("Generating proof (this may take a while)...");
    let result = client.prove(&PROGRAM, stdin).executor(ExecutorKind::Emulator).run()?.await?;
    println!("Proof generated successfully in {:?}", result.get_proving_time());
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

    let output = Output { hash: hash.into(), iterations: n, magic_number: 0xDEADBEEF };
    println!("Expected output hash: {:02x?}", &hash[..8]);

    println!("Verifying saved proofs from disk...");
    let publics = PublicValues::write_abi(&output)?;
    let vk = PROGRAM.vk()?;

    println!("Loading proof with publics from disk...");
    let proof = Proof::load("tmp/sha_hasher_proof.bin")?;

    println!("Verifying proof with embedded publics...");
    // Verify the proof with its embedded publics (from guest's commit)
    proof.with_program_vk(&vk).with_publics(&publics).verify()?;
    println!("Proof verification successful!");

    println!("\u{2713} Successfully generated and verified all proofs!");

    Ok(())
}
