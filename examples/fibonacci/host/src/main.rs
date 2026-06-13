//! ZisK host: loads the guest ELF, writes input, proves execution, verifies
//! the proof, and asserts the committed output matches the expected result.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -- <n> [--asm] [--gpu]
//! ```
//!
//! Where `<n>` is the input as a u8. Defaults to `10` when no
//! argument or wrong argumentis provided.

use std::error::Error;

use fibonacci_common::{U256, fibonacci};
use zisk_sdk::{GuestProgram, ProofKind, ProverClient, ZiskStdin, load_program};

/// Guest ELF binary, embedded into the host at build time.
static PROGRAM: GuestProgram = load_program!("fibonacci-guest");

fn main() -> Result<(), Box<dyn Error>> {
    // Obtaining external input or setting the default one and parsing flags for prover configuration.
    let (input, asm, gpu) = examples_utils::parse_args(10u8);

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

    // One-time setup: derive the proving and verification keys for this guest.
    client.setup(&PROGRAM).run_sync()?;

    // Write the input into the guest's standard input.
    // The type must match what the guest reads: `ziskos::io::read::<u8>()`.
    let stdin = ZiskStdin::new();
    stdin.write::<u8>(&input);

    // Execute the guest and produce a zero-knowledge proof of the run.
    let proof = client.prove(&PROGRAM, stdin).wrap(ProofKind::VadcopFinalMinimal).run_sync()?;

    // Verify the proof against the guest's verification key.
    // Returns an error if the proof is malformed or belongs to a different
    // guest program.
    if proof.with_program_vk(&PROGRAM.vk()?).verify().is_ok() {
        println!("Proof was verified successfully.");
    }

    // Confirm the committed public output matches the locally computed value.
    // The type must match what the guest committed: `ziskos::io::commit(&U256)`.
    let expected_output = fibonacci(input);
    assert_eq!(proof.get_publics().read::<U256>()?, expected_output);

    println!("fibonacci({input}) => {expected_output}");

    Ok(())
}
