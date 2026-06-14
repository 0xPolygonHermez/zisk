//! ZisK host: loads the guest ELF, writes input, proves execution, verifies
//! the proof, and asserts the committed output matches the expected result.
//!
//! # Usage
//!
//! ```text
//! cargo run --release --bin struct -- <n>... [--asm] [--gpu]
//! ```
//!
//! Where `<n>...` is the input as a space-separated list of u64 values.
//! Defaults to `[5, 11, 18, 23, 45]` when no argument is provided.

use primes_common::{is_prime, InputDTO};
use zisk_sdk::{load_program, GuestProgram, ProverClient, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("struct-guest");

fn main() -> anyhow::Result<()> {
    // Obtaining external input or setting the default one and parsing flags for prover configuration.
    let (values, asm, gpu): (Vec<u64>, _, _) = examples_utils::parse_args(vec![5, 11, 18, 23, 45]);
    let input = InputDTO { values };

    // Building the client builder with multiple configurations based on flags.
    // The embedded executor runs entirely in-process; asm/gpu layers add acceleration.
    let mut builder = ProverClient::embedded();
    if asm {
        builder = builder.assembly();
    }
    if gpu {
        builder = builder.gpu();
    }
    let client = builder.build()?;

    // Generating the input.
    let stdin = ZiskStdin::new();
    stdin.write(&input);

    // One-time preprocessing: generates the program setup and the program vertification key
    client.setup(&PROGRAM).run_sync()?;

    // Execute guest and generate the ZK proof
    let proof = client.prove(&PROGRAM, stdin.clone()).run_sync()?;

    // Cryptographic verification of the proof
    if proof.verify().is_ok() {
        println!("Proof was verified successfully.");
    }

    // The committed public must match the value computed locally. The type here
    // must match what the guest committed (`ziskos::io::commit(&u64)`).
    let expected_output = input.values.iter().filter(|n| is_prime(n)).sum::<u64>();
    assert_eq!(proof.get_publics().read::<u64>()?, expected_output);

    println!("sum-primes({:?}) => {expected_output}", input.values);

    Ok(())
}
