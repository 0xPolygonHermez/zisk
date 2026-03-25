use anyhow::Result;
use zisk_sdk::{
    include_guest_elf, EmbeddedGuestElf, GuestProgram, ProofOpts, ProverClient, ZiskStdin,
};

pub const ELF: EmbeddedGuestElf = include_guest_elf!("sha-hasher-guest");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client (Reduced proof mode)...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client...");
    let client = ProverClient::builder().build().unwrap();

    println!("Setting up program...");
    let (pk, _vkey) = client.setup(&GuestProgram::from_elf(ELF))?;
    println!("Setup completed successfully");

    println!("Generating Vadcop proof...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result = client.prove(&pk, stdin).with_proof_options(proof_opts).run()?;
    println!("Vadcop proof generated in {:?}", vadcop_result.get_duration());

    println!("Reducing proof (this may take a while)...");
    let result = client.reduce(vadcop_result.get_proof_with_publics()).run()?;

    // Alternatively, you can also call `minimal()` on the `ProverClient.prove` method to generate a minimal proof directly.
    // let result = client.prove(&pk, stdin).with_proof_options(proof_opts).minimal().run()?;

    println!("Verifying minimal proof...");
    result.verify()?;
    println!("Minimal proof verification successful!");

    println!("\u{2713} Successfully generated and verified minimal proof!");

    Ok(())
}
