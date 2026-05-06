use anyhow::Result;
use examples_common::{build_client, ClientConfig};
use serde::{Deserialize, Serialize};
use zisk_sdk::{load_program, GuestProgram, ZiskStdin};

#[derive(Serialize, Deserialize)]
struct GuestPublics {
    n: u16,
    module: u8,
    b: u32,
}

static PROGRAM1: GuestProgram = load_program!("guest");
static PROGRAM2: GuestProgram = load_program!("guest-agg");

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...\n");

    // Create an input stream and write '2000' to it.
    let n = 2000u16;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    println!("Building prover client...");
    let client = build_client(ClientConfig { minimal_memory: true, ..Default::default() })?;

    println!("Setting up first program...");
    client.upload(&PROGRAM1).run()?;
    client.setup(&PROGRAM1).run()?.await?;

    println!("Setting up second program...");
    client.upload(&PROGRAM2).run()?;
    client.setup(&PROGRAM2).run()?.await?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    println!("Executing first program...");
    let result = client.execute(&PROGRAM1, stdin.clone()).run()?.await?;

    println!(
        "Program executed successfully: {} cycles in {} ms",
        result.get_execution_steps(),
        result.get_execution_time()
    );

    let publics: GuestPublics = result.get_public_values()?;

    let expected_module: u8 = 233;
    let expected_b = {
        let mut a: u32 = 0;
        let mut b: u32 = 1;
        for _ in 0..n {
            let c = (a + b) % expected_module as u32;
            a = b;
            b = c;
        }
        b
    };

    assert_eq!(publics.n, n, "expected n={}, got {}", n, publics.n);
    assert_eq!(
        publics.module, expected_module,
        "expected module={}, got {}",
        expected_module, publics.module
    );
    assert_eq!(publics.b, expected_b, "expected b={}, got {}", expected_b, publics.b);
    println!("Publics OK: n={}, module={}, b={}", publics.n, publics.module, publics.b);

    println!("Generating first proof for program...");
    let vadcop_result1 = client.prove(&PROGRAM1, stdin).run()?.await?;

    let n = 2000u32;
    let stdin2 = ZiskStdin::new();
    stdin2.write(&n);

    println!("Generating second proof for program...");
    let vadcop_result2 = client.prove(&PROGRAM1, stdin2).run()?.await?;

    // Write the proofs, publics, and verification keys to be verified by the guest
    let stdin_aggregation = ZiskStdin::new();

    stdin_aggregation.write(&vadcop_result1.get_proof_bytes());
    stdin_aggregation.write(&vadcop_result2.get_proof_bytes());

    let result_aggregation = client.prove(&PROGRAM2, stdin_aggregation).run()?.await?;

    result_aggregation.verify()?;

    Ok(())
}
