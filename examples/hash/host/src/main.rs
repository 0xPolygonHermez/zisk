//! ZisK host: loads the guest ELF, writes input, proves execution, verifies
//! the proof, and asserts the committed output matches the expected result.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -- <n> [--asm] [--gpu]
//! ```
//!
//! Where `<n>` is the input as a String. Defaults to `Hello Zisk!` when no
//! argument or wrong argumentis provided.

use hash_common::{hex, Digest, Hash, Sha256};
use zisk_sdk::{load_program, GuestProgram, ProverClient, ZiskStdin};

const DEFAULT_INPUT: &str = "Hello Zisk!";

// Embeds the compiled guest ELF at build time
static PROGRAM: GuestProgram = load_program!("hash-guest");

fn main() -> anyhow::Result<()> {
    // Obtaining external input or setting the default one and parsing flags for prover configuration.
    let (input, asm, gpu) = examples_utils::parse_args(DEFAULT_INPUT.to_string());

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
    let proof = client.prove(&PROGRAM, stdin).run_sync()?;

    // Cryptographic verification of the proof
    if proof.with_program_vk(&PROGRAM.vk()?).verify().is_ok() {
        println!("Proof was verified successfully.");
    }

    // The committed public must match the value computed locally. The type here
    // must match what the guest committed (`ziskos::io::commit(&Hash)`).
    let expected_output: Hash = Sha256::digest(&input).into();
    assert_eq!(proof.get_publics().read::<Hash>()?, expected_output);

    println!("sha256('{input}') => 0x{}", hex::encode(expected_output));

    Ok(())
}
