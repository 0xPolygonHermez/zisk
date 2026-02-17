use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zisk_sdk::{
    ZiskStdin, ZiskIO, ElfBinary,
    ProofOpts, ProverClient, ZiskProof, ZiskProofWithPublicValues, ZiskPublics, include_elf,
};

pub const ELF: ElfBinary = include_elf!("sha-hasher-guest");

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
    let client = ProverClient::builder().asm().base_port(54321).build().unwrap();

    println!("Setting up program...");
    client.setup(&ELF)?;
    println!("Setup completed successfully");

    println!("Generating proof (this may take a while)...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let result = client.prove(stdin).with_proof_options(proof_opts).run()?;
    println!("Proof generated successfully in {:?}", result.get_duration());
    println!("Execution steps: {}", result.get_execution_steps());

    println!("Verifying proof...");
    client.verify(result.get_proof(), result.get_publics(), result.get_program_vk())?;
    println!("Proof verification successful!");

    println!("Saving proof to disk...");
    result.save_proof_with_publics("tmp/sha_hasher_proof_with_publics.bin")?;
    result.get_proof().save("tmp/sha_hasher_proof.bin")?;
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
    let proof = ZiskProof::load("tmp/sha_hasher_proof.bin")?;
    let vk = client.vk(&ELF)?;
    println!("Verifying standalone proof...");
    client.verify(&proof, &publics, &vk)?;
    println!("Standalone proof verification successful!");

    println!("Loading proof with publics from disk...");
    let proof_with_publics =
        ZiskProofWithPublicValues::load("tmp/sha_hasher_proof_with_publics.bin")?;
    println!("Verifying proof with publics...");
    client.verify(&proof_with_publics.proof, &proof_with_publics.publics, &vk)?;
    println!("Proof with publics verification successful!");

    println!("\u{2713} Successfully generated and verified all proofs!");

    Ok(())
}
