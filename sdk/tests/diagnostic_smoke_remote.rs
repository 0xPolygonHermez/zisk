//! Remote variant of the `diagnostic` smoke test: drives a running
//! coordinator + workers through `execute` (ASM) + `prove` (ASM).
//! `verify_constraints` is unsupported on `RemoteClient`, so `prove` is the
//! end-to-end signal. The coordinator URL comes from
//! `ZISK_TEST_COORDINATOR_URL` (default below).
//!
//! Lives in its own integration test file — separate from the embedded
//! variant in `diagnostic_smoke.rs` — because the SDK enforces a per-process
//! `ProverClient` singleton (`sdk/src/client.rs`). Each integration test file
//! is its own test binary/process, so keeping the embedded and remote
//! variants apart lets both build a client without tripping the singleton.
//!
//! Ignored by default because it requires a running coordinator + workers.
//! Run with:
//!
//!     cargo test -p zisk-sdk --test diagnostic_smoke_remote -- --ignored --nocapture

use test_artifacts::ELF_DIAGNOSTIC;
use zisk_sdk::{ExecutorKind, ProverClient, ZiskStdin};

/// Coordinator URL for the remote variant, overridable via
/// `ZISK_TEST_COORDINATOR_URL`. Default mirrors `liveness_smoke`.
const DEFAULT_COORDINATOR_URL: &str = "http://127.0.0.1:7000";

fn coordinator_url() -> String {
    std::env::var("ZISK_TEST_COORDINATOR_URL")
        .unwrap_or_else(|_| DEFAULT_COORDINATOR_URL.to_string())
}

#[tokio::test]
#[ignore = "requires a running coordinator + workers; run with --ignored"]
async fn test_diagnostic_remote() {
    let url = coordinator_url();
    eprintln!("[diagnostic_smoke_remote] remote coordinator at {url}");

    let client = ProverClient::remote(&url).build().expect("failed to build RemoteClient");

    // Register the ELF with the coordinator, then set up the ROM.
    client.upload(&ELF_DIAGNOSTIC).run().expect("failed to submit upload");

    client
        .setup(&ELF_DIAGNOSTIC)
        .run()
        .expect("failed to submit setup")
        .await
        .expect("ROM setup failed");

    client
        .execute(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(ExecutorKind::Assembly)
        .run()
        .expect("failed to submit execute")
        .await
        .expect("execute failed");

    client
        .prove(&ELF_DIAGNOSTIC, ZiskStdin::new())
        .executor(ExecutorKind::Assembly)
        .run()
        .expect("failed to submit prove")
        .await
        .expect("prove failed");
}
