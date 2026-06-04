//! End-to-end smoke test driving the `diagnostic` guest ELF through
//! `verify_constraints`. Exercises the executor, every state machine, and
//! every precompile chip in one shot — a single pass/fail signal that the
//! whole stack agrees on a real program.
//!
//! Ignored by default because `verify_constraints` requires a generated
//! proving key on disk and the run is heavy. Execute explicitly with:
//!
//!     cargo test -p zisk-sdk --test diagnostic_smoke_embedded -- --ignored --nocapture
//!
//! `test_diagnostic_embedded` — in-process: execute (ASM + EMU),
//! verify_constraints, prove (ASM). Topology (GPU / CPU+MPI / mac) is
//! selected by how the test is launched, not by the test body.
//!
//! The remote variant lives in its own file (`diagnostic_smoke_remote.rs`)
//! because the SDK enforces a per-process `ProverClient` singleton
//! (`sdk/src/client.rs`); each integration test file is its own process, so
//! splitting the two variants lets both build a client.

use std::path::PathBuf;

use test_artifacts::ELF_DIAGNOSTIC;
use zisk_sdk::{EmbeddedClientBuilder, ExecutorKind, VerifyConstraintsExtension, ZiskStdin};

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn test_diagnostic_embedded() {
    let mut builder = EmbeddedClientBuilder::default().assembly();

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

    client
        .execute(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(ExecutorKind::Emulator)
        .run()
        .expect("failed to submit emulator execute")
        .await
        .expect("emulator execute failed");

    client
        .execute(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .run()
        .expect("failed to submit execute")
        .await
        .expect("execute failed");

    // Run verify_constraints
    client
        .verify_constraints(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(ExecutorKind::Emulator)
        .run()
        .expect("failed to submit verify_constraints")
        .await
        .expect("verify_constraints failed");

    client
        .prove(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(ExecutorKind::Assembly)
        .run()
        .expect("failed to submit prove")
        .await
        .expect("prove failed");
}
