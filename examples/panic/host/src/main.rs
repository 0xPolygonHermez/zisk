use anyhow::Result;
use zisk_sdk::{load_program, GuestProgram, ProverClient, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("panic-guest");

#[tokio::main]
async fn main() -> Result<()> {
    let client = ProverClient::remote("http://127.0.0.1:7000").build()?;

    client.upload(&PROGRAM).run()?;
    client.setup(&PROGRAM).run()?.await?;

    // Valid execution
    let stdin = ZiskStdin::new();
    stdin.write(&42u64);
    println!("Executing with value 42 (valid)...");
    let outcome: Result<_> =
        async { client.execute(&PROGRAM, stdin).run()?.await }.await;
    match outcome {
        Ok(r) => println!("  ok: {} cycles in {} ms", r.get_execution_steps(), r.get_execution_time()),
        Err(e) => println!("  err: {e:#}"),
    }

    // Panic failure
    let stdin = ZiskStdin::new();
    stdin.write(&0u64);
    println!("Executing with value 0 (panic)...");
    let outcome: Result<_> =
        async { client.execute(&PROGRAM, stdin).run()?.await }.await;
    match outcome {
        Ok(r) => println!("  ok: {} cycles in {} ms", r.get_execution_steps(), r.get_execution_time()),
        Err(e) => println!("  err: {e:#}"),
    }

    // Assert failure
    let stdin = ZiskStdin::new();
    stdin.write(&1u64);
    println!("Executing with value 1 (assert)...");
    let outcome: Result<_> =
        async { client.execute(&PROGRAM, stdin).run()?.await }.await;
    match outcome {
        Ok(r) => println!("  ok: {} cycles in {} ms", r.get_execution_steps(), r.get_execution_time()),
        Err(e) => println!("  err: {e:#}"),
    }

    // Segfault
    let stdin = ZiskStdin::new();
    stdin.write(&2u64);
    println!("Executing with value 2 (segfault)...");
    let outcome: Result<_> =
        async { client.execute(&PROGRAM, stdin).run()?.await }.await;
    match outcome {
        Ok(r) => println!("  ok: {} cycles in {} ms", r.get_execution_steps(), r.get_execution_time()),
        Err(e) => println!("  err: {e:#}"),
    }

    // Valid execution again
    let stdin = ZiskStdin::new();
    stdin.write(&99u64);
    println!("Executing with value 99 (valid)...");
    let outcome: Result<_> =
        async { client.execute(&PROGRAM, stdin).run()?.await }.await;
    match outcome {
        Ok(r) => println!("  ok: {} cycles in {} ms", r.get_execution_steps(), r.get_execution_time()),
        Err(e) => println!("  err: {e:#}"),
    }

    Ok(())
}
