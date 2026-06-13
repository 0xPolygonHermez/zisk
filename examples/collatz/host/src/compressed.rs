//! ZisK host: loads the guest ELF, writes input, proves execution, verifies
//! the proof, and asserts the committed output matches the expected result.
//!
//! # Usage
//!
//! ```text
//! cargo run --release --bin compressed -- <n> [--asm] [--gpu]
//! ```
//!
//! Where `<n>` is the starting value as a u64. Defaults to `55` when no
//! argument or wrong argument is provided.

use std::error::Error;

use collatz_common::{Digest, Hash, Sha256, collatz, hex};
use zisk_sdk::{GuestProgram, ProverClient, ZiskStdin, load_program};

static PROGRAM: GuestProgram = load_program!("compressed-guest");

fn main() -> Result<(), Box<dyn Error>> {
    // Obtaining external input or setting the default one and parsing flags for prover configuration.
    let (input, asm, gpu) = examples_utils::parse_args(55u64);

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
    if proof.with_program_vk(&PROGRAM.vk()?).verify().is_ok() {
        println!("Proof was verified successfully.");
    }

    // The committed public must match the value computed locally. The type here
    // must match what the guest committed (`ziskos::io::commit(&Hash)`).
    let sequence = collatz(input);
    let mut hasher = Sha256::new();
    hasher.update(input.to_le_bytes());
    sequence.iter().for_each(|value| hasher.update(value.to_le_bytes()));
    let expected_output: Hash = hasher.finalize().into();
    assert_eq!(proof.get_publics().read::<Hash>()?, expected_output);

    println!("sha256('{input}', collatz('{input}')) => 0x{}", hex::encode(expected_output));

    Ok(())
}
