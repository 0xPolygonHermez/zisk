use std::path::PathBuf;

use anyhow::Result;
use zisk_common::io::ZiskStdin;
use zisk_sdk::{ProverClient, include_elf};

/// The ELF we want to execute inside the zkVM.
const ELF: &[u8] = include_elf!("sha-hasher-guest");
// const ELF_INFO: (&[u8], ElfInfo) = include_elf!("sha-hasher-guest", with_info);

fn main() -> Result<()> {
    // // Setup logging.
    // utils::setup_logger();
    println!("Starting ZisK Prover Client...");
    // Create an input stream and write '1000' to it.
    let n = 1000u32;

    let filename = env!(concat!("ZISK_ELF_", "sha-hasher-guest"));

    // The input stream that the program will read from using `sp1_zkvm::io::read`. Note that the
    // types of the elements in the input stream must match the types being read in the program.
    // TODO! Use ZiskMemoryStdin
    let stdin = ZiskStdin::from_vec(n.to_le_bytes().to_vec());

    // Create a `ProverClient` method.
    let client = ProverClient::builder()
        .emu()
        .prove()
        .witness_lib_path(PathBuf::from("/home/xavi/dev/zisk/target/release/libzisk_witness.so"))
        .proving_key_path(PathBuf::from("/home/xavi/dev/zisk/build/pk13"))
        .elf_path(PathBuf::from(filename)) // TODO! change to elf bytes
        .build()
        .unwrap();

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(stdin)?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.execution.executed_steps, result.duration
    );

    // // Generate the proof for the given program and input.
    // let (pk, vk) = client.setup(ELF);
    // let mut proof = client.prove(&pk, &stdin).plonk().run().unwrap();
    // let stdin = ZiskStdin::from_vec(n.to_le_bytes().to_vec());
    // let result = client.prove(stdin)?;

    // Read and verify the output.
    //
    // Note that this output is read from values committed to in the program using
    // `sp1_zkvm::io::commit`.
    // let _ = proof.public_values.read::<u32>();
    // let a = proof.public_values.read::<u32>();
    // let b = proof.public_values.read::<u32>();

    // println!("a: {}", a);
    // println!("b: {}", b);

    // // Verify proof and public values
    // client.verify(&proof, &vk).expect("verification failed");

    // // Test a round trip of proof serialization and deserialization.
    // proof.save("proof-with-pis.bin").expect("saving proof failed");
    // let deserialized_proof =
    //     SP1ProofWithPublicValues::load("proof-with-pis.bin").expect("loading proof failed");

    // // Verify the deserialized proof.
    // client.verify(&deserialized_proof, &vk).expect("verification failed");

    // println!("successfully generated and verified proof for the program!")

    Ok(())
}
