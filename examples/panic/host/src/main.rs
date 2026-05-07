use anyhow::Result;
use zisk_sdk::{load_program, GuestProgram, ProverClient, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("panic-guest");

#[tokio::main]
async fn main() -> Result<()> {
    let client = ProverClient::remote("http://127.0.0.1:7000").build()?;

    client.upload(&PROGRAM).run()?;
    client.setup(&PROGRAM).run()?.await?;

    let stdin1 = ZiskStdin::new();
    stdin1.write(&1u64);
    println!("Executing with value 1...");
    let outcome: Result<_> =
        async { client.execute(&PROGRAM, stdin1).run()?.await }.await;
    match outcome {
        Ok(r) => println!("  ok: {} cycles in {} ms", r.get_execution_steps(), r.get_execution_time()),
        Err(e) => println!("  err: {e:#}"),
    }

    let stdin2 = ZiskStdin::new();
    stdin2.write(&0u64);
    println!("Executing with value 0...");
    let outcome: Result<_> =
        async { client.execute(&PROGRAM, stdin2).run()?.await }.await;
    match outcome {
        Ok(r) => println!("  ok: {} cycles in {} ms", r.get_execution_steps(), r.get_execution_time()),
        Err(e) => println!("  err: {e:#}"),
    }

    let stdin3 = ZiskStdin::new();
    stdin3.write(&1u64);
    println!("Executing with value 1...");
    let outcome: Result<_> =
        async { client.execute(&PROGRAM, stdin3).run()?.await }.await;
    match outcome {
        Ok(r) => println!("  ok: {} cycles in {} ms", r.get_execution_steps(), r.get_execution_time()),
        Err(e) => println!("  err: {e:#}"),
    }

    Ok(())
}
