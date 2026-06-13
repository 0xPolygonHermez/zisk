//! ZisK host: loads the guest ELF, writes input, proves execution, verifies
//! the proof, and asserts the committed output matches the expected result.
//!
//! # Usage
//!
//! ```text
//! cargo run --release --bin remote -- <a> <b>
//! ```
//!
//! Where `<a>` and `<b>` are the two input u64 values. Defaults to `5 10` when
//! no argument or wrong argument is provided.

use std::error::Error;

use gcd_common::gcd;
use zisk_sdk::{GuestProgram, ProverClient, ZiskStdin, load_program};

static PROGRAM: GuestProgram = load_program!("gcd-guest");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Obtaining external input or setting the default one. This binary proves
    // against a remote prover, so the `--asm`/`--gpu` flags do not apply here.
    let (input, _, _): ((u64, u64), _, _) = examples_utils::parse_args((5, 10));

    // Build a client that offloads proving to a remote prover at the given URL.
    // `connect_timeout` bounds the initial connection; `request_timeout` bounds
    // each request, which must be generous enough to cover proof generation.
    let client = ProverClient::remote("http://localhost:7000")
        .connect_timeout(std::time::Duration::from_secs(10))
        .request_timeout(std::time::Duration::from_secs(2000))
        .build()?;

    // Generating the input.
    let stdin = ZiskStdin::new();
    stdin.write(&input.0);
    stdin.write(&input.1);

    // One-time preprocessing: generates the program setup and the program vertification key
    client.setup(&PROGRAM).run()?.await?;

    // Execute guest and generate the ZK proof
    let proof = client.prove(&PROGRAM, stdin.clone()).run()?.await?;

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
