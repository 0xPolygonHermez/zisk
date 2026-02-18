use anyhow::Result;
use zisk_sdk::{
    ElfBinary, ProverClient, ZiskIO, ZiskProofWithPublicValues, ZiskStdin, include_elf,
};

pub const ELF: ElfBinary = include_elf!("sha-hasher-guest");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client (SNARK mode)...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client with SNARK support...");
    let client = ProverClient::builder().asm().base_port(54321).snark().build().unwrap();

    println!("Setting up program and generating verification key...");
    let vkey = client.setup(&ELF)?;
    println!("Setup completed successfully");

    println!("Generating PLONK proof (this may take a while)...");
    let snark_proof = client.prove(stdin).plonk().run()?;
    println!("PLONK proof generated successfully in {:?}", snark_proof.get_duration());
    println!("Execution steps: {}", snark_proof.get_execution_steps());

    // Alternatively, it can also be done in two steps
    // let vadcop_result = client.prove(stdin).run()?;
    // let snark_proof = client.prove_snark(&vadcop_result.get_proof(), &vadcop_result.get_publics(), &vkey)?;

    println!("Verifying PLONK proof...");
    client.verify(snark_proof.get_proof(), snark_proof.get_publics(), &vkey)?;
    println!("PLONK proof verification successful!");

    println!("Saving PLONK proof to disk...");
    snark_proof.save_proof_with_publics("/tmp/sha_hasher_proof_snark_with_publics.bin")?;
    println!("Proof saved to /tmp/sha_hasher_proof_snark_with_publics.bin");

    println!("Loading and verifying saved PLONK proof...");
    let proof = ZiskProofWithPublicValues::load("/tmp/sha_hasher_proof_snark_with_publics.bin")?;
    let vk = client.vk(&ELF)?;
    client.verify(proof.get_proof(), proof.get_publics(), &vk)?;
    println!("Saved PLONK proof verification successful!");

    println!("\u{2713} Successfully generated and verified PLONK proof!");

    Ok(())
}
