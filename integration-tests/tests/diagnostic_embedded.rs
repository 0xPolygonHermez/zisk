//! End-to-end smoke test driving the `diagnostic` guest ELF through
//! `verify_constraints`. Exercises the executor, every state machine, and
//! every precompile chip in one shot — a single pass/fail signal that the
//! whole stack agrees on a real program.
//!
//! Ignored by default because `verify_constraints` requires a generated
//! proving key on disk and the run is heavy. Execute explicitly with:
//!
//!     cargo test -p integration-tests --test diagnostic_embedded -- --ignored --nocapture
//!
//! `test_diagnostic_embedded` — in-process: execute, verify_constraints,
//! prove. On Linux the Assembly executor is built and the body additionally
//! exercises the ASM execute and ASM prove paths; on macOS (no asm/jit support)
//! the test runs the Emulator path end-to-end (EMU execute, verify_constraints,
//! EMU prove). Topology (GPU / CPU+MPI / mac) is selected by how the test is
//! launched, not by the test body.
//!
//! The remote variant lives in its own file (`diagnostic_remote.rs`) because
//! the SDK enforces a per-process `ProverClient` singleton
//! (`sdk/src/client.rs`); each integration test file is its own process, so
//! splitting the two variants lets both build a client.

use std::path::PathBuf;

use test_artifacts::ELF_DIAGNOSTIC;
use zisk_sdk::{
    EmbeddedClientBuilder, ExecutorKind, VerifyConstraintsExtension, WitnessBuilderExt, ZiskStdin,
};

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn test_diagnostic_embedded() {
    // On Linux build the Assembly executor: an Asm-built prover serves both the
    // ASM and EMU paths, so the body can exercise both per-operation. macOS has
    // no asm/jit support, so it builds the Emulator prover and runs the EMU path
    // end-to-end instead.
    let mut builder = EmbeddedClientBuilder::default();
    #[cfg(target_os = "linux")]
    {
        builder = builder.assembly();
    }

    if std::env::var_os("ZISK_TEST_NO_AGGREGATION").is_some() {
        eprintln!("[diagnostic_smoke] ZISK_TEST_NO_AGGREGATION set — disabling aggregation");
        builder = builder.no_aggregation();
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

    client
        .execute(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(ExecutorKind::Emulator)
        .run()
        .expect("failed to submit emulator execute")
        .await
        .expect("emulator execute failed");

    // ASM execute — Linux only (no asm/jit support on macOS).
    #[cfg(target_os = "linux")]
    client
        .execute(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(ExecutorKind::Assembly)
        .run()
        .expect("failed to submit asm execute")
        .await
        .expect("asm execute failed");

    // Run verify_constraints
    client
        .verify_constraints(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(ExecutorKind::Emulator)
        .run()
        .expect("failed to submit verify_constraints")
        .await
        .expect("verify_constraints failed");

    // Prove via Assembly on Linux; macOS falls back to the Emulator executor,
    // which an Emu-built prover serves end-to-end.
    #[cfg(target_os = "linux")]
    let prove_executor = ExecutorKind::Assembly;

    #[cfg(not(target_os = "linux"))]
    let prove_executor = ExecutorKind::Emulator;
    client
        .prove(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(prove_executor)
        .run()
        .expect("failed to submit prove")
        .await
        .expect("prove failed");
}
