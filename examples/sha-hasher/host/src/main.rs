use anyhow::Result;
use zisk_sdk::{load_program, GuestProgram, ProofOpts, ProverClient, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    let client = ProverClient::builder().asm().build().unwrap();

    client.setup(&PROGRAM).run()?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(&PROGRAM, stdin.clone())?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.get_execution_steps(),
        result.get_duration()
    );

    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result = client.prove(&PROGRAM, stdin).with_proof_options(proof_opts).run()?;

    let vkey = client.vk(&PROGRAM)?;
    vadcop_result.program_vk(&vkey).verify()?;

    println!("successfully generated and verified proof for the program!");

    Ok(())
}
