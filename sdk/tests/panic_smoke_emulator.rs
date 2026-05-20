//! Smoke test for guest failure propagation through the embedded client,
//! using the emulator executor. Runs the `panic_modes` guest with each
//! failure-mode input and asserts that the SDK surfaces the failure as an
//! error from `execute().await`, while valid inputs succeed — including the
//! post-failure case, which exercises in-process recovery.
//!
//! Lives in its own integration test file because the SDK enforces a
//! per-process `ProverClient` singleton (`sdk/src/client.rs`); the assembly
//! variant lives in `panic_smoke_assembly.rs` and runs in a separate
//! process.
//!
//! Run: cargo test -p zisk-sdk --test panic_smoke_emulator -- --ignored --nocapture

use std::path::PathBuf;

use test_artifacts::ELF_PANIC_MODES;
use zisk_sdk::{EmbeddedClient, EmbeddedClientBuilder, ZiskStdin};

async fn execute_with_input(client: &EmbeddedClient, input: u64) -> anyhow::Result<()> {
    let stdin = ZiskStdin::new();
    stdin.write(&input);
    client.execute(&ELF_PANIC_MODES, stdin).run()?.await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn panic_modes_emulator() {
    eprintln!("[panic_smoke_emulator] using Emulator executor");

    let mut builder = EmbeddedClientBuilder::default();
    if let Some(pk) = std::env::var_os("ZISK_TEST_PROVING_KEY").map(PathBuf::from) {
        eprintln!("[panic_smoke_emulator] using ZISK_TEST_PROVING_KEY={}", pk.display());
        builder = builder.proving_key(pk);
    } else {
        eprintln!("[panic_smoke_emulator] no override set, using ZiskPaths default");
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
        eprintln!("[panic_smoke_emulator] input={input} ({label}) errored as expected");
    }

    // Post-failure: valid input must succeed, proving in-process recovery.
    execute_with_input(&client, 99).await.expect("post-failure valid execute");
}
