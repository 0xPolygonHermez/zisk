use anyhow::Result;

use zisk_sdk::{load_program, EmbeddedOpts, GuestProgram, ProverClient, ZiskStdin};

static PROGRAM1: GuestProgram = load_program!("multiple-program-guest");
static PROGRAM2: GuestProgram = load_program!("multiple-program-guest-2");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    // let client = ProverClient::embedded().build()?;

    let embedded_opts = EmbeddedOpts::default().minimal_memory();
    let builder = ProverClient::embedded().with_embedded_opts(embedded_opts);
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    println!("Setting up first program...");
    client.upload(&PROGRAM1).run()?;
    client.setup(&PROGRAM1).run()?.await?;

    println!("Setting up second program...");
    client.upload(&PROGRAM2).run()?;
    client.setup(&PROGRAM2).run()?.await?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing first program...");
    let result = client.execute(&PROGRAM1, stdin.clone()).run()?.await?;

    println!(
        "Program executed successfully: {} cycles in {} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    println!("Generating proof for first program...");
    let vadcop_result = client.prove(&PROGRAM1, stdin).run()?.await?;

    println!("Verifying proof...");
    let vkey = PROGRAM1.vk()?;
    vadcop_result.with_program_vk(&vkey).verify()?;
    println!("Successfully generated and verified proof for first program!\n");

    let n = 2000u32;
    let stdin2 = ZiskStdin::new();
    stdin2.write(&n);

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing second program...");
    let result2 = client.execute(&PROGRAM2, stdin2.clone()).run()?.await?;

    println!(
        "Program executed successfully: {} cycles in {} ms",
        result2.get_execution_steps(),
        result2.get_execution_time()
    );

    println!("Generating proof for second program...");
    let vadcop_result2 = client.prove(&PROGRAM2, stdin2).run()?.await?;

    println!("Verifying proof...");
    let vkey2 = PROGRAM2.vk()?;
    vadcop_result2.with_program_vk(&vkey2).verify()?;
    println!("Successfully generated and verified proof for second program!\n");

    println!("All proofs generated and verified successfully!");

    Ok(())
}
