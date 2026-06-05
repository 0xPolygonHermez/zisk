//! Smoke test for guest failure propagation through the embedded client,
//! using the emulator executor. Runs the `panic_modes` guest with each
//! failure-mode input and asserts that the SDK surfaces the failure as an
//! error from `execute()`, while valid inputs succeed — including the
//! post-failure case, which exercises in-process recovery.
//!
//! Lives in its own integration test file because the SDK enforces a
//! per-process `ProverClient` singleton (`sdk/src/client.rs`); the assembly
//! variant lives in `panic_smoke_assembly.rs` and runs in a separate
//! process.
//!
//! Run: cargo test -p zisk-sdk --test panic_smoke_emulator -- --ignored --nocapture

use test_artifacts::{ELF_MISSING_ENTRYPOINT, ELF_PANIC_MODES};
use zisk_sdk::{EmbeddedClientBuilder, EmbeddedExecuteOnlyClient, ZiskStdin};

fn execute_with_input(client: &EmbeddedExecuteOnlyClient, input: u64) -> anyhow::Result<()> {
    let stdin = ZiskStdin::new();
    stdin.write(&input);
    client.execute(&ELF_PANIC_MODES, stdin, None)?;
    Ok(())
}

#[test]
#[ignore]
fn panic_modes_emulator() {
    eprintln!("[panic_smoke_emulator] using Emulator executor");

    let client = EmbeddedClientBuilder::default()
        .emulator()
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
        eprintln!("[panic_smoke_emulator] input={input} ({label}) errored as expected");
    }

    // Post-failure: valid input must succeed, proving in-process recovery.
    execute_with_input(&client, 99).expect("post-failure valid execute");
}

#[test]
fn rejects_elf_without_entrypoint_macro() {
    let err = zisk_sdk::run(&ELF_MISSING_ENTRYPOINT, ZiskStdin::new(), None)
        .expect_err("elf2rom should reject a guest ELF that has no entry point");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("entry point") && msg.contains("ziskos::entrypoint!"),
        "expected actionable entrypoint error, got: {msg}"
    );
}
