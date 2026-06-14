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

use gcd_common::gcd;
use zisk_sdk::{load_program, EmbeddedOpts, GuestProgram, ProverClient, ZiskStdin};

static PROGRAM: GuestProgram = load_program!("gcd-guest");

fn main() -> anyhow::Result<()> {
    // Obtaining external input or setting the default one and parsing flags for prover configuration.
    let (input, asm, gpu): ((u64, u64), _, _) = examples_utils::parse_args((5, 10));

    // Building the client builder with multiple configurations based on flags.
    // The embedded executor runs entirely in-process; asm/gpu layers add acceleration.
    let mut builder = ProverClient::embedded();
    if asm {
        builder = builder.assembly();
    }
    if gpu {
        builder = builder.gpu();
    }

    // Tuning the embedded executor. Here every field is spelled out explicitly,
    // but the same all-defaults struct can be obtained with
    // `let mut opts = EmbeddedOpts::default();`.
    let mut opts = EmbeddedOpts {
        minimal_memory: false,
        proving_key: None,
        proving_key_snark: None,
        preload_plonk: false,
        max_witness_stored: None,
        number_threads_witness: None,
        max_streams: None,
    };

    // Minimize memory usage; the remaining None fields let ZisK pick sensible defaults.
    opts = opts.minimal_memory();

    // Build the client with the embedded options applied.
    let client = builder.with_embedded_opts(opts).build()?;

    // Generating the input.
    let stdin = ZiskStdin::new();
    stdin.write(&input.0);
    stdin.write(&input.1);

    // One-time preprocessing: generates the program setup and the program vertification key
    client.setup(&PROGRAM).run_sync()?;

    // Execute guest and generate the ZK proof
    let proof = client.prove(&PROGRAM, stdin.clone()).run_sync()?;

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
