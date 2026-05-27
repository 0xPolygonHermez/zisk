//! Smoke test for guest failure propagation through the embedded client,
//! using the assembly executor. Mirror of `panic_smoke_emulator` but with
//! `.assembly()` on the builder.
//!
//! Linux-only — the asm executor depends on mmap/jit support not available
//! on macOS/Windows. On other platforms this file compiles to an empty test
//! binary.
//!
//! Lives in its own integration test file because the SDK enforces a
//! per-process `ProverClient` singleton (`sdk/src/client.rs`); the emulator
//! variant lives in `panic_smoke_emulator.rs` and runs in a separate
//! process.
//!
//! Run: cargo test -p zisk-sdk --test panic_smoke_assembly -- --ignored --nocapture

#![cfg(target_os = "linux")]

use std::path::PathBuf;

use test_artifacts::{ELF_MISSING_ENTRYPOINT, ELF_PANIC_MODES};
use zisk_sdk::{EmbeddedClient, EmbeddedClientBuilder, ZiskStdin};

async fn execute_with_input(client: &EmbeddedClient, input: u64) -> anyhow::Result<()> {
    let stdin = ZiskStdin::new();
    stdin.write(&input);
    client.execute(&ELF_PANIC_MODES, stdin).run()?.await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn panic_modes_assembly() {
    eprintln!("[panic_smoke_assembly] using Assembly executor");

    let mut builder = EmbeddedClientBuilder::default().assembly();
    if let Some(pk) = std::env::var_os("ZISK_TEST_PROVING_KEY").map(PathBuf::from) {
        eprintln!("[panic_smoke_assembly] using ZISK_TEST_PROVING_KEY={}", pk.display());
        builder = builder.proving_key(pk);
    } else {
        eprintln!("[panic_smoke_assembly] no override set, using ZiskPaths default");
    }

    let client = builder.build().expect("failed to build EmbeddedClient");

    client
        .setup(&ELF_PANIC_MODES)
        .run()
        .expect("failed to submit setup")
        .await
        .expect("ROM setup failed");

    // Baseline: valid input must succeed before exercising the failure modes.
    execute_with_input(&client, 42).await.expect("baseline valid execute");

    // Each failure-mode input must surface as Err.
    for (input, label) in [(0u64, "panic"), (1u64, "assert"), (2u64, "segfault")] {
        let outcome = execute_with_input(&client, input).await;
        assert!(outcome.is_err(), "expected input={input} ({label}) to surface as Err, got Ok",);
        eprintln!("[panic_smoke_assembly] input={input} ({label}) errored as expected");
    }

    // Post-failure: valid input must succeed, proving in-process recovery.
    execute_with_input(&client, 99).await.expect("post-failure valid execute");
}

#[test]
fn rejects_elf_without_entrypoint_macro() {
    let err = ELF_MISSING_ENTRYPOINT
        .run_emulation(zisk_common::io::ZiskStdin::new(), None)
        .expect_err("elf2rom should reject a guest ELF that has no entry point");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("entry point") && msg.contains("ziskos::entrypoint!"),
        "expected actionable entrypoint error, got: {msg}"
    );
}
