use anyhow::Result;
use serde::{Deserialize, Serialize};
use test_artifacts::{ELF_AGG_VERIFY, ELF_FIB_MOD};
use zisk_sdk::{EmbeddedOpts, ProfilingMode, ProverClient, ZiskStdin};

#[derive(Serialize, Deserialize)]
struct GuestPublics {
    n: u32,
    module: u32,
    b: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    let n: u32 = 2000;
    let module: u32 = 233;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    stdin.write(&module);

    let embedded_opts = EmbeddedOpts::default().minimal_memory();
    let builder = ProverClient::embedded().with_embedded_opts(embedded_opts);
    #[cfg(feature = "gpu")]
    let builder = builder.gpu();
    let client = builder.build()?;

    println!("Setting up first program (fib_mod)...");
    client.setup(&ELF_FIB_MOD).run()?.await?;

    println!("Setting up second program (agg_verify)...");
    client.setup(&ELF_AGG_VERIFY).run()?.await?;

    println!("Executing first program...");
    let result = client.execute(&ELF_FIB_MOD, &stdin).run()?.await?;

    println!(
        "Program executed successfully: {} cycles in {} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    let publics: GuestPublics = result.get_public_values()?;

    let expected_b = {
        let mut a: u32 = 0;
        let mut b: u32 = 1;
        for _ in 0..n {
            let c = (a + b) % module;
            a = b;
            b = c;
        }
        b
    };

    assert_eq!(publics.n, n, "expected n={}, got {}", n, publics.n);
    assert_eq!(publics.module, module, "expected module={}, got {}", module, publics.module);
    assert_eq!(publics.b, expected_b, "expected b={}, got {}", expected_b, publics.b);
    println!("Publics OK: n={}, module={}, b={}", publics.n, publics.module, publics.b);

    println!("Generating first proof for fib_mod...");
    let vadcop_result1 = client.prove(&ELF_FIB_MOD, stdin).run()?.await?;

    let stdin2 = ZiskStdin::new();
    stdin2.write(&n);
    stdin2.write(&module);

    println!("Generating second proof for fib_mod...");
    let vadcop_result2 = client.prove(&ELF_FIB_MOD, stdin2).run()?.await?;

    let stdin_aggregation = ZiskStdin::new();
    stdin_aggregation.write_slice(&vadcop_result1.get_proof_bytes()?);
    stdin_aggregation.write_slice(&vadcop_result2.get_proof_bytes()?);

    println!("Running ZisK Emulator on aggregation program for profiling...");
    zisk_sdk::run(&ELF_AGG_VERIFY, stdin_aggregation.clone(), Some(ProfilingMode::Complete))?;

    let result_aggregation = client.prove(&ELF_AGG_VERIFY, stdin_aggregation).run()?.await?;

    result_aggregation.verify()?;

    Ok(())
}
