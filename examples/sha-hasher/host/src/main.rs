use std::path::PathBuf;

use anyhow::Result;
use zisk_common::io::ZiskIO;
use zisk_common::io::ZiskStdin;
use zisk_sdk::{ProofOpts, ProverClient, ZiskProveResult, include_elf};

pub const ELF: &str = include_elf!("sha-hasher-guest");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    let client = ProverClient::builder()
        .proving_key_path(PathBuf::from("/home/roger/zisk/build/provingKey"))
        .build()
        .unwrap();

    let vk = client.setup(ELF)?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(stdin.clone())?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.execution.executed_steps, result.duration
    );

    let proof_opts = ProofOpts::default().minimal_memory();
    let proof = client.prove(stdin).with_proof_options(proof_opts).run()?;
    client.verify(&proof, &vk)?;

    proof.save("/tmp/sha_hasher_proof_snark.bin")?;

    let proof_stored = ZiskProveResult::load("/tmp/sha_hasher_proof_snark.bin")?;
    client.verify(&proof_stored, &vk)?;

    println!("successfully generated and verified proof for the program!");

    Ok(())
}
