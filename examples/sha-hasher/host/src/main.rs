use anyhow::Result;
use zisk_sdk::{load_program, ExecutorKind, GuestProgram, ProverClient, ProverOpts, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    let proof_opts = ProverOpts::default().minimal_memory();
    let client =
        ProverClient::embedded().with_prover_options(proof_opts).gpu().assembly().build()?;

    client.setup(&PROGRAM).run()?.await?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result =
        client.execute(&PROGRAM, stdin.clone()).executor(ExecutorKind::Assembly).run()?.await?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    let vadcop_result =
        client.prove(&PROGRAM, stdin).executor(ExecutorKind::Assembly).run()?.await?;

    let vkey = PROGRAM.vk()?;
    vadcop_result.with_program_vk(&vkey).verify()?;

    println!("successfully generated and verified proof for the program!");

    Ok(())
}
