use anyhow::Result;
use zisk_sdk::{
    include_guest_elf, EmbeddedGuestElf, GuestProgram, ProofOpts, ProverClient, ZiskStdin,
};

pub const ELF: EmbeddedGuestElf = include_guest_elf!("multiple-program-guest");
pub const ELF2: EmbeddedGuestElf = include_guest_elf!("multiple-program-guest-2");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    let client = ProverClient::builder().build().unwrap();

    println!("Setting up first program...");
    let (pk, vkey) = client.setup(&GuestProgram::from_elf(ELF))?;

    println!("Setting up second program...");
    let (pk2, vkey2) = client.setup(&GuestProgram::from_elf(ELF2))?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing first program...");
    let result = client.execute(&pk, stdin.clone())?;

    println!(
        "Program executed successfully: {} cycles in {:.2?}",
        result.get_execution_steps(),
        result.get_duration()
    );

    println!("Generating proof for first program...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result = client.prove(&pk, stdin).with_proof_options(proof_opts).run()?;

    println!("Verifying proof...");
    vadcop_result.program_vk(&vkey).verify()?;
    println!("Successfully generated and verified proof for first program!\n");

    let n = 2000u32;
    let stdin2 = ZiskStdin::new();
    stdin2.write(&n);

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing second program...");
    let result2 = client.execute(&pk2, stdin2.clone())?;

    println!(
        "Program executed successfully: {} cycles in {:.2?}",
        result2.get_execution_steps(),
        result2.get_duration()
    );

    println!("Generating proof for second program...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result2 = client.prove(&pk2, stdin2).with_proof_options(proof_opts).run()?;

    println!("Verifying proof...");
    vadcop_result2.program_vk(&vkey2).verify()?;
    println!("Successfully generated and verified proof for second program!\n");

    println!("All proofs generated and verified successfully!");

    Ok(())
}
