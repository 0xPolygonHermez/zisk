use anyhow::Result;
use zisk_sdk::{load_program, GuestProgram, ProverClient, ProverOpts, ZiskStdin};

static PROGRAM1: GuestProgram = load_program!("guest");
static PROGRAM2: GuestProgram = load_program!("guest-agg");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    let proof_opts = ProverOpts::default().minimal_memory();
    let client = ProverClient::embedded().with_prover_options(proof_opts).gpu().build()?;

    println!("Setting up first program...");
    client.setup(&PROGRAM1).run()?.await?;

    println!("Setting up second program...");
    client.setup(&PROGRAM2).run()?.await?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing first program...");
    let result = client.execute(&PROGRAM1, stdin.clone()).run()?.await?;

    println!(
        "Program executed successfully: {} cycles in {:.2?}",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    println!("Generating first proof for program...");
    let vadcop_result1 = client.prove(&PROGRAM1, stdin).run()?.await?;

    let n = 2000u32;
    let stdin2 = ZiskStdin::new();
    stdin2.write(&n);

    println!("Generating second proof for program...");
    let vadcop_result2 = client.prove(&PROGRAM1, stdin2).run()?.await?;

    // Write the proofs, publics, and verification keys to be verified by the guest
    let stdin_aggregation = ZiskStdin::new();

    stdin_aggregation.write(&vadcop_result1.get_proof_bytes());
    stdin_aggregation.write(&vadcop_result2.get_proof_bytes());

    let result_aggregation = client.prove(&PROGRAM2, stdin_aggregation).run()?.await?;

    result_aggregation.verify()?;

    Ok(())
}
