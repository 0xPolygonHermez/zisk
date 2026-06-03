use anyhow::Result;
use test_artifacts::ELF_FIB_MOD;
use zisk_sdk::{EmbeddedOpts, ProverClient, ZiskStdin};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    // Prove the same parameterized fib_mod ELF twice, with different (n, module) inputs.
    // Demonstrates a host orchestrating multiple proofs in one run.
    let stdin = ZiskStdin::new();
    stdin.write(&1000u32);
    stdin.write(&233u32);

    let embedded_opts = EmbeddedOpts::default().minimal_memory();
    let builder = ProverClient::embedded().with_embedded_opts(embedded_opts);
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    println!("Setting up program (single ELF, two invocations)...");
    client.upload(&ELF_FIB_MOD).run()?;
    client.setup(&ELF_FIB_MOD).run()?.await?;

    println!("Executing first invocation (n=1000, module=233)...");
    let result = client.execute(&ELF_FIB_MOD, &stdin).run()?.await?;
    println!(
        "Program executed successfully: {} cycles in {} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    println!("Generating proof for first invocation...");
    let vadcop_result = client.prove(&ELF_FIB_MOD, &stdin).run()?.await?;

    println!("Verifying proof...");
    let vkey = ELF_FIB_MOD.vk()?;
    vadcop_result.with_program_vk(&vkey).verify()?;
    println!("Successfully generated and verified first proof!\n");

    let stdin2 = ZiskStdin::new();
    stdin2.write(&2000u32);
    stdin2.write(&253u32);

    println!("Executing second invocation (n=2000, module=253)...");
    let result2 = client.execute(&ELF_FIB_MOD, &stdin2).run()?.await?;
    println!(
        "Program executed successfully: {} cycles in {} ms",
        result2.get_execution_steps(),
        result2.get_execution_time()
    );

    println!("Generating proof for second invocation...");
    let vadcop_result2 = client.prove(&ELF_FIB_MOD, &stdin2).run()?.await?;

    println!("Verifying proof...");
    vadcop_result2.with_program_vk(&vkey).verify()?;
    println!("Successfully generated and verified second proof!\n");

    println!("All proofs generated and verified successfully!");

    Ok(())
}
