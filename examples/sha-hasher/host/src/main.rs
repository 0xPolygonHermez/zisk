use anyhow::Result;
use sha_hasher_host::ELF_SHA_HASHER;
use zisk_sdk::{ExecutorKind, ProverClient, ZiskStream};

#[tokio::main]
async fn main() -> Result<()> {
    let coordinator_url =
        std::env::var("ZISK_COORDINATOR_URL").unwrap_or_else(|_| "http://127.0.0.1:15100".into());
    let client = ProverClient::remote(coordinator_url).build()?;

    client.upload(&ELF_SHA_HASHER).run()?;

    let setup_handle = client.setup(&ELF_SHA_HASHER).run()?;
    setup_handle.await?;

    let input = ZiskStream::unix();

    let handle =
        client.execute(&ELF_SHA_HASHER, input.clone()).executor(ExecutorKind::Assembly).run()?;
    input.write(&1000u32);
    input.flush()?;
    let result = handle.await?;

    println!(
        "ZisK has executed program with {} cycles in {} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    input.write(&2000u32);
    let prove_handle =
        client.prove(&ELF_SHA_HASHER, input.clone()).executor(ExecutorKind::Assembly).run()?;
    input.flush()?;
    let vadcop_result = prove_handle.await?;

    let vkey = ELF_SHA_HASHER.vk()?;
    vadcop_result.with_program_vk(&vkey).verify()?;

    println!("successfully generated and verified proof for the program!");
    println!("Running second proof generation with new input...");

    input.write(&3000u32);
    let prove_handle2 =
        client.prove(&ELF_SHA_HASHER, input.clone()).executor(ExecutorKind::Assembly).run()?;
    input.flush()?;
    let vadcop_result2 = prove_handle2.await?;

    let vkey = ELF_SHA_HASHER.vk()?;
    vadcop_result2.with_program_vk(&vkey).verify()?;

    println!("successfully generated and verified proof for the program!");

    Ok(())
}
