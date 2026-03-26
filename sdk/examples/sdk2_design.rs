//! SDK2 design example — demonstrates the full prover client API.

use std::time::Duration;

use anyhow::Result;
use zisk_sdk::{
    EmbeddedOptions, Executor, GuestProgram, ProverClient, PublicValues, RemoteOptions, Tracing,
    VerificationKey, WatchEvent, ZiskHints, ZiskStdin,
};

pub static PROGRAM: GuestProgram = zisk_sdk::load_program!("guest");

#[tokio::main]
async fn main() -> Result<()> {
    // Alternative: load at runtime from a URI (file path or http(s)://)
    let _program_from_uri = GuestProgram::from_uri("./guest.elf")?;

    // Input / Hints

    let stdin = ZiskStdin::new();
    let _stdin_stream = ZiskStdin::stream("unix:///tmp/stdin.sock")?;
    let hints = ZiskHints::stream("unix:///tmp/hints.sock")?;
    let _hints_file = ZiskHints::stream("/home/user/hints.bin")?;

    let embedded_options = EmbeddedOptions::default();
    let _embedded_client = ProverClient::embedded(embedded_options).gpu().build()?;

    let remote_options = RemoteOptions::builder().url("localhost:3000").build()?;
    let _remote_client =
        ProverClient::remote(remote_options).gpu().executor(Executor::Assembly).build()?;

    // Default: embedded + emulator, no GPU.
    let client = ProverClient::default();

    // --- Setup ----------------------------------------------------------------

    // Embedded → no-op. Remote → uploads ELF, registers program on coordinator.
    client.upload(&PROGRAM).run()?;

    // Embedded → executes ROM setup locally.
    // Remote   → enables program for proving on coordinator.
    client.setup(&PROGRAM).run()?;

    // With hints enabled in ROM setup (requires Assembly).
    // client.setup(&PROGRAM).with_hints().run()?;

    // --- Basic API (sync) -------------------------------------------------------
    // run() blocks the calling thread until the proof is ready.
    // Internally wraps submit() + proof().await inside a Tokio runtime.
    // No async required from the caller — suitable for most use cases.

    let proof = client.prove(&PROGRAM, stdin.clone()).stark().run()?;

    let pv = PublicValues(vec![]);
    let vk = VerificationKey(vec![]);

    assert!(proof.verify().is_ok(), "Proof verification failed");
    assert!(
        proof.verify_with().publics(&pv).verification_key(&vk).run().is_ok(),
        "Public values and verification key verification failed"
    );

    // --- Advanced API (async) ---------------------------------------------------
    // submit() returns a ProofHandle immediately without blocking.
    // Useful when the caller needs to watch events, prove multiple programs
    // concurrently, or integrate into an existing async runtime.
    // Requires an async context (e.g. #[tokio::main]).

    let on_event = |event: WatchEvent| {
        println!("{:?}", event);
    };

    let handle = client
        .prove(&PROGRAM, stdin.clone())
        .executor(Executor::Assembly)
        .minimal_memory()
        .stark()
        .hints(hints)
        .timeout(Duration::from_secs(60))
        .on(WatchEvent::All, on_event) // pre-submit callback
        .submit()?;

    // Post-submit: same .on() — missed events are not replayed.
    // Use handle.proof().await as the reliable completion path.
    handle.on(WatchEvent::All, |event| println!("{:?}", event));

    let proof = handle.proof().await?;

    assert!(proof.verify().is_ok(), "Proof verification failed");
    assert!(
        proof.verify_with().publics(&pv).verification_key(&vk).run().is_ok(),
        "Public values and verification key verification failed"
    );

    // --- Execute (no proof) ---------------------------------------------------

    let _result = client
        .execute(&PROGRAM, stdin)
        .timeout(Duration::from_secs(5))
        .trace(Tracing::Input)
        .trace(Tracing::Hints)
        .run()?;

    Ok(())
}
