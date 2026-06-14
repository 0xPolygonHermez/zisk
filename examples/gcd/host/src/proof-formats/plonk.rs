//! ZisK host: loads the guest ELF, writes input, proves execution, verifies
//! the proof, and asserts the committed output matches the expected result.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -- <a> <b> [--asm] [--gpu]
//! ```
//!
//! Where `<a>` and `<b>` are the two input u64 values. Defaults to `5 10` when
//! no argument or wrong argument is provided.

use std::error::Error;

use gcd_common::gcd;
use zisk_sdk::{GuestProgram, ProofKind, ProverClient, ZiskStdin, load_program};

static PROGRAM: GuestProgram = load_program!("gcd-guest");

fn main() -> Result<(), Box<dyn Error>> {
    // Obtaining external input or setting the default one and parsing flags for prover configuration.
    let (input, asm, gpu): ((u64, u64), _, _) = examples_utils::parse_args((5, 10));

    // Building the client builder with multiple configurations based on flags.
    // The embedded executor runs entirely in-process; asm/gpu layers add acceleration.
    let mut builder = ProverClient::embedded().plonk();
    if asm {
        builder = builder.assembly();
    }
    if gpu {
        builder = builder.gpu();
    }
    let client = builder.build()?;

    // Generating the input.
    let stdin = ZiskStdin::new();
    stdin.write(&input.0);
    stdin.write(&input.1);

    // One-time preprocessing: generates the program setup and the program vertification key
    client.setup(&PROGRAM).run_sync()?;

    // Execute guest and generate the ZK proof and wrap it into a constant-size, on-chain-verifiable proof.
    let proof = client.prove(&PROGRAM, stdin).wrap(ProofKind::Plonk).run_sync()?;

    // Cryptographic verification of the proof
    if proof.with_program_vk(&PROGRAM.vk()?).verify().is_ok() {
        println!("Proof was verified successfully.");
    }

    // The committed public must match the value computed locally. The type here
    // must match what the guest committed (`ziskos::io::commit(&u64)`).
    let expected_output = gcd(input.0, input.1);
    assert_eq!(proof.get_publics().read::<u64>()?, expected_output);

    println!("gcd({:?}, {:?}) => {expected_output}", input.0, input.1);

    Ok(())
}
