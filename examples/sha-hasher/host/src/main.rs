use std::path::PathBuf;

use anyhow::Result;
use zisk_common::io::ZiskIO;
use zisk_common::io::ZiskStdin;
use zisk_sdk::{ProofOpts, ProverClient};

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

    let elf_path = PathBuf::from(
        "/home/roger/zisk/examples/target/riscv64ima-zisk-zkvm-elf/release/sha-hasher-guest",
    );

    let vk = client.setup(elf_path.clone())?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(stdin.clone())?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.execution.executed_steps, result.duration
    );

    let proof_opts = ProofOpts::default().verify_proofs();
    let proof = client.prove(stdin).with_proof_options(proof_opts).compressed().run()?;
    client.verify(&proof, &vk).expect("verification failed");

    proof.save("sha_hasher_proof.bin")?;

    println!("successfully generated and verified proof for the program!");

    Ok(())
}
