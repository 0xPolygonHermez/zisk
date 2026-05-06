#[cfg(feature = "remote")]
compile_error!(
    "the `hints` example does not support the `remote` feature — \
     hints require the Assembly executor, which on remote depends on coordinator \
     configuration and cannot be selected by the client. Run without `--features remote`."
);

use anyhow::Result;
use zisk_sdk::{GuestProgram, ProverClient, ZiskHints, ZiskStdin};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    let elf_path = "hints/example/zec-reth.elf";
    let program = GuestProgram::from_uri(elf_path)?;

    let hints_path = "hints/example/24654300_hints.bin";
    let hints = ZiskHints::from_file(hints_path)?;

    let builder = ProverClient::embedded();
    #[cfg(feature = "asm")]
    let builder = builder.assembly();
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    println!("Setting up program...");
    client.upload(&program).run()?;
    client.setup(&program).with_hints().run()?.await?;

    println!("Executing program...");
    let result = client.execute(&program, ZiskStdin::new()).hints(hints).run()?.await?;

    println!(
        "Program executed successfully: {} cycles in {} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    Ok(())
}
