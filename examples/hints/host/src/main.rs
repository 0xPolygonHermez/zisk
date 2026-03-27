use anyhow::Result;
use zisk_sdk::{EmbeddedOptions, ExecutorKind, GuestProgram, ProverClient, ZiskHints};

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    let elf_path = "hints/example/zec-reth.elf";
    let hints_path = "hints/example/24654300_hints.bin";

    let program = GuestProgram::from_uri(&elf_path)?;
    let hints = ZiskHints::file(&hints_path)?;

    // Create a `ProverClient` method.
    let embedded_options = EmbeddedOptions::default();
    let client = ProverClient::embedded(embedded_options).assembly().build()?;

    println!("Setting up program...");
    client.setup(&program).with_hints().run()?;


    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing program...");
    let result = client.execute(&program, hints).executor(ExecutorKind::Assembly).run()?;

    println!(
        "Program executed successfully: {} cycles in {:.2?}",
        result.get_execution_steps(),
        result.get_duration()
    );

    Ok(())
}
