use anyhow::Result;

use zisk_sdk::{load_program, GuestProgram, ProofOpts, ProverClient, ZiskStdin};

static PROGRAM1: GuestProgram = load_program!("multiple-program-guest");
static PROGRAM2: GuestProgram = load_program!("multiple-program-guest-2");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    let client = ProverClient::builder().build().unwrap();

    println!("Setting up first program...");
    client.setup(&PROGRAM1).run()?;

    println!("Setting up second program...");
    client.setup(&PROGRAM2).run()?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing first program...");
    let result = client.execute(&PROGRAM1, stdin.clone())?;

    println!(
        "Program executed successfully: {} cycles in {:.2?}",
        result.get_execution_steps(),
        result.get_duration()
    );

    println!("Generating proof for first program...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result = client.prove(&PROGRAM1, stdin).with_proof_options(proof_opts).run()?;

    println!("Verifying proof...");
    let vkey = client.vk(&PROGRAM1)?;
    vadcop_result.program_vk(&vkey).verify()?;
    println!("Successfully generated and verified proof for first program!\n");

    let n = 2000u32;
    let stdin2 = ZiskStdin::new();
    stdin2.write(&n);

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing second program...");
    let result2 = client.execute(&PROGRAM2, stdin2.clone())?;

    println!(
        "Program executed successfully: {} cycles in {:.2?}",
        result2.get_execution_steps(),
        result2.get_duration()
    );

    println!("Generating proof for second program...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result2 = client.prove(&PROGRAM2, stdin2).with_proof_options(proof_opts).run()?;

    println!("Verifying proof...");
    let vkey2 = client.vk(&PROGRAM2)?;
    vadcop_result2.program_vk(&vkey2).verify()?;
    println!("Successfully generated and verified proof for second program!\n");

    println!("All proofs generated and verified successfully!");

    Ok(())
}
