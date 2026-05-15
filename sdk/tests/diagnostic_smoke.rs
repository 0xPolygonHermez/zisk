//! End-to-end smoke test driving the `diagnostic` guest ELF through
//! `verify_constraints`. Exercises the executor, every state machine, and
//! every precompile chip in one shot — a single pass/fail signal that the
//! whole stack agrees on a real program.
//!
//! Ignored by default because `verify_constraints` requires a generated
//! proving key on disk and the run is heavy. Execute explicitly with:
//!
//!     cargo test -p zisk-sdk --test diagnostic_smoke -- --ignored --nocapture
//!

use std::path::PathBuf;

use test_artifacts::ELF_DIAGNOSTIC;
use zisk_sdk::{EmbeddedClientBuilder, VerifyConstraintsExtension, ZiskStdin};

// Look for a proving key in the local `build/provingKey` directory
fn local_build_proving_key() -> Option<PathBuf> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()?.to_path_buf();
    let candidate = workspace_root.join("build").join("provingKey");
    candidate.is_dir().then_some(candidate)
}

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn verify_constraints_diagnostic() {
    let mut builder = EmbeddedClientBuilder::default();

    // Get the proving key
    if let Some(pk) = local_build_proving_key() {
        eprintln!("[diagnostic_smoke] using local proving key at {}", pk.display());
        builder = builder.proving_key(pk);
    } else {
        eprintln!("[diagnostic_smoke] no build/provingKey found, falling back to default");
    }

    // Build the client
    let client = builder.build().expect("failed to build EmbeddedClient");

    // Setup the program
    client
        .setup(&ELF_DIAGNOSTIC)
        .run()
        .expect("failed to submit setup")
        .await
        .expect("ROM setup failed");

    // Run verify_constraints
    client
        .verify_constraints(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .run()
        .expect("failed to submit verify_constraints")
        .await
        .expect("verify_constraints failed");
}
