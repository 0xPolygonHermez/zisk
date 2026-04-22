use anyhow::Result;
use zisk_sdk::{ExecutorKind, GuestProgram, ProverClient, ZiskHints, ZiskStdin};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    let elf_path = "hints/example/zec-reth.elf";
    let hints_path = "hints/example/24654300_hints.bin";

    let program = GuestProgram::from_uri(elf_path)?;
    let hints = ZiskHints::file(hints_path)?;

    // Create a `ProverClient` method.
    let client = ProverClient::remote("http://127.0.0.1:7000").build()?;

    println!("Setting up program...");
    client.upload(&program).run()?;
    client.setup(&program).with_hints().run()?.await?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing program...");
    let result = client.execute(&program, ZiskStdin::new())
        .hints(hints)
        .executor(ExecutorKind::Assembly)
        .run()?
        .await?;

    println!(
        "Program executed successfully: {} cycles in {:.2?} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    Ok(())
}
