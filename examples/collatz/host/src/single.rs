//! ZisK host: loads the guest ELF, writes input, proves execution, verifies
//! the proof, and asserts the committed output matches the expected result.
//!
//! # Usage
//!
//! ```text
//! cargo run --release --bin struct -- <n> [--asm] [--gpu]
//! ```
//!
//! Where `<n>` is the starting value as a u64. Defaults to `55` when no
//! argument or wrong argument is provided.

use collatz_common::{collatz, OutputDTO};
use zisk_sdk::{load_program, GuestProgram, ProverClient, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("single-guest");

fn main() -> anyhow::Result<()> {
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
    // must match what the guest committed (`ziskos::io::commit(&OutputDTO)`).
    let expected_result = OutputDTO { n: input, sequence: collatz(input) };
    assert_eq!(proof.get_publics().read::<OutputDTO>()?, expected_result);

    println!("collatz({input}) => {0:?}", expected_result.sequence);

    Ok(())
}
