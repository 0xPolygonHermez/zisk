//! ZisK host: loads the guest ELF, writes input, proves execution, verifies
//! the proof, and asserts the committed output matches the expected result.
//!
//! # Usage (the assembly prover is required for streaming)
//!
//! ```text
//! cargo run --release --bin streaming -- <n>... [--gpu]
//! ```
//!
//! Where `<n>...` is the input as a space-separated list of u64 values.
//! Defaults to `[5, 11, 18, 23, 45]` when no argument is provided.

use std::error::Error;

use primes_common::is_prime;
use zisk_sdk::{GuestProgram, ProverClient, ZiskStream, load_program};

static PROGRAM: GuestProgram = load_program!("multiple-guest");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Obtaining external input or setting the default one and parsing flags for prover configuration.
    let (input, _asm, gpu): (Vec<u64>, _, _) = examples_utils::parse_args(vec![5, 11, 18, 23, 45]);

    // Building the client builder with multiple configurations based on flags.
    // Streaming requires the assembly prover layer, so it is always enabled here;
    // the gpu layer adds further acceleration when requested.
    let mut builder = ProverClient::embedded().assembly();
    if gpu {
        builder = builder.gpu();
    }
    let client = builder.build()?;

    // Generating the input stream and writing the length of the input as the first value.
    let stream = ZiskStream::unix();
    stream.write(&(input.len() as u64));

    // One-time preprocessing: generates the program setup and the program vertification key
    client.setup(&PROGRAM).run_sync()?;

    // Launch the guest execution to begin generating the ZK proof
    let job = client.prove(&PROGRAM, stream.clone()).run()?;

    // Writing the input values to the stream one by one, flushing after each write to ensure
    for value in &input {
        stream.write::<u64>(value);
        stream.flush()?;
    }

    //  Await the completion of the guest execution and obtain the generated proof
    let proof = job.await?;

    // Cryptographic verification of the proof
    if proof.verify().is_ok() {
        println!("Proof was verified successfully.");
    }

    // The committed public must match the value computed locally. The type here
    // must match what the guest committed.
    let expected_output = input.iter().filter(|n| is_prime(n)).sum::<u64>();
    assert_eq!(proof.get_publics().read::<u64>()?, expected_output);

    println!("sum-primes({:?}) => {expected_output}", input);

    Ok(())
}
