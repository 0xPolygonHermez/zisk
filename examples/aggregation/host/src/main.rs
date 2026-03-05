use anyhow::Result;
use zisk_sdk::{include_elf, ElfBinary, ProofOpts, ProverClient, ZiskIO, ZiskStdin};

pub const ELF: ElfBinary = include_elf!("guest");
pub const ELF2: ElfBinary = include_elf!("guest-agg");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    let client = ProverClient::builder().build().unwrap();

    println!("Setting up first program...");
    let (pk, vkey) = client.setup(&ELF)?;

    println!("Setting up second program...");
    let (pk2, vkey2) = client.setup(&ELF2)?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing first program...");
    let result = client.execute(&pk, stdin.clone())?;

    println!(
        "Program executed successfully: {} cycles in {:.2?}",
        result.get_execution_steps(),
        result.get_duration()
    );

    println!("Generating first proof for program...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result1 = client.prove(&pk, stdin).with_proof_options(proof_opts).run()?;

    let n = 2000u32;
    let stdin2 = ZiskStdin::new();
    stdin2.write(&n);

    println!("Generating second proof for program...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result2 = client.prove(&pk, stdin2).with_proof_options(proof_opts).run()?;

    // Write the proofs, publics, and verification keys to be verified by the guest
    let stdin_aggregation = ZiskStdin::new();

    let proof1 = client.prepare_send_proof(
        &vadcop_result1.get_proof(),
        &vadcop_result1.get_publics(),
        &vkey,
    )?;
    let proof2 = client.prepare_send_proof(
        &vadcop_result2.get_proof(),
        &vadcop_result2.get_publics(),
        &vkey,
    )?;

    stdin_aggregation.write_proof(&proof1);
    stdin_aggregation.write_proof(&proof2);

    let proof_opts = ProofOpts::default().minimal_memory();

    let result_aggregation =
        client.prove(&pk2, stdin_aggregation).with_proof_options(proof_opts).run()?;

    client.verify(result_aggregation.get_proof(), result_aggregation.get_publics(), &vkey2)?;

    Ok(())
}
