use anyhow::Result;
use zisk_sdk::{load_program, ExecutorKind, GuestProgram, ProverClient, ZiskStream};

static PROGRAM: GuestProgram = load_program!("sha-hasher-guest");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    let client = ProverClient::remote("http://127.0.0.1:7000").build()?;

    client.upload(&PROGRAM).run()?;
    client.setup(&PROGRAM).run()?.await?;

    let input = ZiskStream::grpc();

    let handle = client.execute(&PROGRAM, input.clone()).executor(ExecutorKind::Assembly).run()?;
    input.write(&1000u32);
    input.flush()?;
    let result = handle.await?; // automatically calls finish() on the stream

    println!(
        "ZisK has executed program with {} cycles in {:?} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    // write → run() → flush() → await()  (finish is automatic on await)
    input.write(&2000u32);
    let prove_handle =
        client.prove(&PROGRAM, input.clone()).executor(ExecutorKind::Assembly).run()?;
    input.flush()?;
    let vadcop_result = prove_handle.await?;

    let vkey = PROGRAM.vk()?;
    vadcop_result.with_program_vk(&vkey).verify()?;

    println!("successfully generated and verified proof for the program!");
    println!("Running second proof generation with new input...");

    input.write(&3000u32);
    let prove_handle2 =
        client.prove(&PROGRAM, input.clone()).executor(ExecutorKind::Assembly).run()?;
    input.flush()?;
    let vadcop_result2 = prove_handle2.await?;

    let vkey = PROGRAM.vk()?;
    vadcop_result2.with_program_vk(&vkey).verify()?;

    println!("successfully generated and verified proof for the program!");

    Ok(())
}
