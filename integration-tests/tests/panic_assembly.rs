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
//! variant lives in `panic_emulator.rs` and runs in a separate process.
//!
//! Run: cargo test -p integration-tests --test panic_assembly -- --ignored --nocapture

#![cfg(target_os = "linux")]

use test_artifacts::ELF_PANIC_MODES;
use zisk_sdk::{EmbeddedClientBuilder, EmbeddedExecuteOnlyClient, ZiskStdin};

fn execute_with_input(client: &EmbeddedExecuteOnlyClient, input: u64) -> anyhow::Result<()> {
    let stdin = ZiskStdin::new();
    stdin.write(&input);
    client.execute(&ELF_PANIC_MODES, stdin, None)?;
    Ok(())
}

#[test]
#[ignore]
fn panic_modes_assembly() {
    eprintln!("[panic_smoke_assembly] using Assembly executor");

    let client = EmbeddedClientBuilder::default()
        .assembly()
        .execute_only()
        .build()
        .expect("failed to build EmbeddedExecuteOnlyClient");

    client.setup(&ELF_PANIC_MODES, false).expect("ROM setup failed");

    // Baseline: valid input must succeed before exercising the failure modes.
    execute_with_input(&client, 42).expect("baseline valid execute");

    // Each failure-mode input must surface as Err.
    for (input, label) in [(0u64, "panic"), (1u64, "assert"), (2u64, "segfault")] {
        let outcome = execute_with_input(&client, input);
        assert!(outcome.is_err(), "expected input={input} ({label}) to surface as Err, got Ok",);
        eprintln!("[panic_smoke_assembly] input={input} ({label}) errored as expected");
    }

    // Post-failure: valid input must succeed, proving in-process recovery.
    execute_with_input(&client, 99).expect("post-failure valid execute");
}
