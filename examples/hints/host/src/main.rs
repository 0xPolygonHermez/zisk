use anyhow::Result;
use std::thread;
use zisk_sdk::{ExecutorKind, GuestProgram, ProverClient, ZiskStdin, ZiskStream};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    let elf_path = "hints/example/zec-reth.elf";
    let hints_path = "hints/example/24654300_hints.bin";

    let program = GuestProgram::from_uri(elf_path)?;

    // Create a QUIC stream for hints.  Port 0 lets the OS pick a free port;
    // ZiskStream resolves it to the actual bound address so the coordinator
    // receives the correct port every run.
    let hints_stream = ZiskStream::quic("quic://127.0.0.1:0")?;
    let s = hints_stream.clone();
    let hints_data = std::fs::read(hints_path)?;
    thread::spawn(move || {
        s.write_raw(&hints_data);
        s.flush().unwrap();
        s.finish().unwrap();
    });

    let client = ProverClient::remote("http://127.0.0.1:7000").build()?;

    println!("Setting up program...");
    client.upload(&program).run()?;
    client.setup(&program).with_hints().run()?.await?;

    println!("Executing program...");
    let result = client
        .execute(&program, ZiskStdin::new())
        .hints(hints_stream)
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
