use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zisk_common::io::ZiskIO;
use zisk_common::io::ZiskStdin;
use zisk_common::ElfBinary;
use zisk_sdk::{ProofOpts, ProverClient, ZiskProof, ZiskPublics, include_elf};

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

    // Create a `ProverClient` method.
    let client = ProverClient::builder()
        .asm()
        .base_port(54321)
        .proving_key_path(PathBuf::from("/home/roger/zisk/build/provingKey"))
        .proving_key_snark_path(PathBuf::from("/home/roger/zisk/build/provingKeySnark"))
        .snark()
        .build()
        .unwrap();

    let vkey = client.setup(&ELF)?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(stdin.clone())?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.execution.executed_steps, result.duration
    );

    let proof_opts = ProofOpts::default().minimal_memory();
    let result = client.prove(stdin).with_proof_options(proof_opts).plonk().run()?;
    client.verify(&result.proof, &result.publics, &vkey)?;

    result.proof.save("/tmp/sha_hasher_proof_snark.bin")?;

    let output: Output = result.get_publics()?;
    println!("Deserialized public outputs: {:?}", output);
    println!("Hash: {:02x?}", output.hash);
    println!("Iterations: {}", output.iterations);
    println!("Magic number: 0x{:08x}", output.magic_number);

    let mut hash = [0u8; 32];
    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }
    let output = Output { hash, iterations: n, magic_number: 0xDEADBEEF };
    let publics = ZiskPublics::write(&output)?;
    let proof = ZiskProof::load("/tmp/sha_hasher_proof_snark.bin")?;
    let vk = client.vk(&ELF)?;
    client.verify(&proof, &publics, &vk)?;

    println!("successfully generated and verified proof for the program!");

    Ok(())
}
