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

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn verify_constraints_diagnostic() {
    let mut builder = EmbeddedClientBuilder::default();

    if std::env::var_os("ZISK_TEST_ASM").is_some() {
        eprintln!("[diagnostic_smoke] ZISK_TEST_ASM set — using Assembly executor");
        builder = builder.assembly();
    }
    if std::env::var_os("ZISK_TEST_GPU").is_some() {
        eprintln!("[diagnostic_smoke] ZISK_TEST_GPU set — enabling GPU");
        builder = builder.gpu();
    }

    if let Some(pk) = std::env::var_os("ZISK_TEST_PROVING_KEY").map(PathBuf::from) {
        eprintln!("[diagnostic_smoke] using ZISK_TEST_PROVING_KEY={}", pk.display());
        builder = builder.proving_key(pk);
    } else {
        eprintln!("[diagnostic_smoke] no override set, using ZiskPaths default");
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
