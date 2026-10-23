use sha_hasher_host::ELF_SHA_HASHER;
use std::error::Error;
use zisk_sdk::{Proof, ProofKind, ProverClient, ZiskStdin};

fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting ZisK Prover Client (pilfflonk SNARK mode)...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client with pilfflonk SNARK support...");
    let builder = ProverClient::embedded().pilfflonk();
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    println!("Setting up program and generating verification key...");
    client.setup(&ELF_SHA_HASHER).run_sync()?;
    println!("Setup completed successfully");

    println!("Generating pilfflonk proof (this may take a while)...");
    let snark_proof = client.prove(&ELF_SHA_HASHER, stdin).wrap(ProofKind::Plonk).run_sync()?;
    println!("pilfflonk proof generated successfully in {} ms", snark_proof.get_proving_time());
    println!("Execution steps: {}", snark_proof.get_execution_steps());

    println!("Verifying pilfflonk proof...");
    snark_proof.verify()?;
    println!("pilfflonk proof verification successful!");

    println!("Saving pilfflonk proof to disk...");
    snark_proof.save_proof("/tmp/sha_hasher_proof_pilfflonk.bin")?;
    println!("Proof saved to /tmp/sha_hasher_proof_pilfflonk.bin");

    println!("Loading and verifying saved pilfflonk proof...");
    let proof = Proof::load("/tmp/sha_hasher_proof_pilfflonk.bin")?;
    let vkey = ELF_SHA_HASHER.vk()?;
    proof.with_program_vk(&vkey).verify()?;
    println!("Saved pilfflonk proof verification successful!");

    println!("\u{2713} Successfully generated and verified pilfflonk proof!");

    Ok(())
}
