//! Liveness smoke test: exercises the worker's post-failure recovery path
//! by running a sequence of operations against a coordinator at
//! `http://127.0.0.1:7000`. After each failure/cancel scenario the next
//! operation MUST succeed — that's what proves the worker came back online.
//!
//! Sequence:
//!   [1] execute valid                  baseline
//!   [2] prove valid, cancel after 2 s  cancel during Contributions → recovery
//!   [3] execute valid                  worker recovered from cancel
//!   [4] prove valid, cancel in Prove   cancel during Prove phase → recovery
//!   [5] execute valid                  worker recovered from prove-phase cancel
//!   [6] execute invalid (bad record)   guest panics on truncated read → recovery
//!   [7] execute valid                  worker recovered from failure
//!   [8] execute panic_modes (panic!)   explicit guest panic! → recovery
//!   [9] execute valid                  worker recovered from explicit panic
//!
//! Prereqs: a coordinator + at least one worker running locally on
//! `127.0.0.1:7000` (override with `ZISK_TEST_COORDINATOR_URL`) and configured
//! to use the assembly executor.
//!
//! Run: cargo test -p zisk-sdk --test liveness_smoke -- --ignored --nocapture
//!
//! **Remote-only by design.** Unlike `panic_smoke` (which uses the embedded
//! client because it only checks error propagation), this test's purpose is
//! to exercise the *worker's* post-failure recovery — the cancel-then-reuse
//! and panic-then-reuse paths only exist in the distributed setup. There is
//! no "worker" to recover in embedded mode, so the test stays remote.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use test_artifacts::{ELF_LIVENESS, ELF_PANIC_MODES};
use tokio::sync::Notify;
use zisk_sdk::{ExecutorKind, JobEvent, ProverClient, RemoteClient, ZiskStdin, ZiskStream};

/// SDK progress percent emitted when the worker enters the prove phase.
/// Mirrors `PROGRESS_PROVE` in `sdk/src/job_handle.rs` — the SDK doesn't
/// expose a phase enum, only `JobEvent::Progress(pct)`.
const PROGRESS_PROVE: u8 = 75;

const DEFAULT_COORDINATOR_URL: &str = "http://127.0.0.1:7000";
const CANCEL_DELAY: Duration = Duration::from_secs(2);

/// After a cancel/failure the worker spends a moment parked in `SettingUp`
/// while it does ASM soft-reset and the recovery handshake. The dispatcher
/// rejects new work during that window with `Code::Unavailable`. We pace each
/// follow-up submission with a short fixed wait, then a few quick retries —
/// observed recovery time on a single-rank local worker is well under 1 s.
const POST_RECOVERY_GRACE: Duration = Duration::from_secs(1);
const SETTLE_RETRY_INTERVAL: Duration = Duration::from_millis(500);
const SETTLE_RETRY_ATTEMPTS: u32 = 20; // ≈10 s total beyond the grace

fn coordinator_url() -> String {
    std::env::var("ZISK_TEST_COORDINATOR_URL")
        .unwrap_or_else(|_| DEFAULT_COORDINATOR_URL.to_string())
}

#[tokio::test]
#[ignore = "requires a running coordinator + worker; run with --ignored"]
async fn liveness_smoke() -> Result<()> {
    let url = coordinator_url();
    eprintln!("[liveness_smoke] coordinator at {url}");
    let client = ProverClient::remote(&url).build()?;

    eprintln!("=== Liveness test ===");
    eprintln!("Uploading + setting up guest programs …");
    client.upload(&ELF_LIVENESS).run()?;
    client.setup(&ELF_LIVENESS).run()?.await?;
    client.upload(&ELF_PANIC_MODES).run()?;
    client.setup(&ELF_PANIC_MODES).run()?.await?;
    eprintln!("Setup done.\n");

    eprintln!("[1/9] Execute valid (baseline)");
    run_valid_execute(&client, /*after_recovery=*/ false).await?;

    eprintln!(
        "\n[2/9] Prove valid, cancel after {}s (Contributions phase)",
        CANCEL_DELAY.as_secs()
    );
    run_prove_with_cancel(&client).await?;

    eprintln!("\n[3/9] Execute valid (after Contributions cancel — worker should have recovered)");
    run_valid_execute(&client, /*after_recovery=*/ true).await?;

    eprintln!("\n[4/9] Prove valid, cancel in Prove phase");
    run_prove_with_cancel_in_prove_phase(&client).await?;

    eprintln!("\n[5/9] Execute valid (after Prove cancel — worker should have recovered)");
    run_valid_execute(&client, /*after_recovery=*/ true).await?;

    eprintln!("\n[6/9] Execute invalid (malformed input — guest panics on truncated read)");
    run_invalid_execute(&client).await?;

    eprintln!("\n[7/9] Execute valid (after invalid — worker should have recovered)");
    run_valid_execute(&client, /*after_recovery=*/ true).await?;

    eprintln!("\n[8/9] Execute panic_modes input=0 (explicit guest panic!)");
    run_panic_execute(&client).await?;

    eprintln!("\n[9/9] Execute valid (after explicit panic — worker should have recovered)");
    run_valid_execute(&client, /*after_recovery=*/ true).await?;

    eprintln!("\n=== All scenarios passed ===");
    Ok(())
}

/// Detect the coordinator's "workers are setting up; retry shortly" response.
fn is_cluster_setting_up(e: &anyhow::Error) -> bool {
    let s = format!("{e:#}");
    s.contains("workers are setting up") || s.contains("Cluster unavailable")
}

/// Wait for the worker(s) to clear `SettingUp` after a cancel/failure, then
/// submit. First sleeps a short grace, then re-tries at most a handful of
/// times. Each attempt prints its outcome so a hang shows up as observable
/// progress instead of a silent stall.
async fn submit_after_recovery<F, T>(label: &str, mut submit: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    eprintln!("  waiting {POST_RECOVERY_GRACE:?} for worker recovery to settle …");
    tokio::time::sleep(POST_RECOVERY_GRACE).await;
    for attempt in 1..=SETTLE_RETRY_ATTEMPTS {
        match submit() {
            Ok(t) => {
                if attempt > 1 {
                    eprintln!("  cluster ready (attempt {attempt}), dispatched {label}");
                }
                return Ok(t);
            }
            Err(e) if is_cluster_setting_up(&e) => {
                eprintln!(
                    "  attempt {attempt}: cluster still settling, sleeping {SETTLE_RETRY_INTERVAL:?} …"
                );
                tokio::time::sleep(SETTLE_RETRY_INTERVAL).await;
            }
            Err(e) => return Err(e),
        }
    }
    anyhow::bail!("{label}: cluster never became ready after {SETTLE_RETRY_ATTEMPTS} attempts");
}

/// Short execute: writes (mode=0, value=42), expects success in ms.
/// Pass `after_recovery=true` if the previous step was a cancel or failure;
/// the helper waits for the worker to clear `SettingUp` before submitting.
async fn run_valid_execute(client: &RemoteClient, after_recovery: bool) -> Result<()> {
    let input = ZiskStream::unix();
    let submit =
        || client.execute(&ELF_LIVENESS, input.clone()).executor(ExecutorKind::Assembly).run();
    let handle =
        if after_recovery { submit_after_recovery("execute", submit).await? } else { submit()? };
    input.write(&0u64); // mode = short
    input.write(&42u64); // value
    input.flush()?;

    let started = Instant::now();
    let result = handle.await?;
    eprintln!(
        "  ok — steps={}, host_elapsed_ms={}, exec_time_ms={}",
        result.get_execution_steps(),
        started.elapsed().as_millis(),
        result.get_execution_time(),
    );
    Ok(())
}

/// Submit a long-running prove (mode=1) and cancel it after CANCEL_DELAY.
/// The prove handle's await must resolve as Cancelled / Err — anything else
/// indicates the cancel didn't propagate.
async fn run_prove_with_cancel(client: &RemoteClient) -> Result<()> {
    let input = ZiskStream::unix();
    let mut handle =
        client.prove(&ELF_LIVENESS, input.clone()).executor(ExecutorKind::Assembly).run()?;
    input.write(&1u64); // mode = long
    input.write(&7u64); // value (seed for the busy-loop)
    input.flush()?;

    let job_id = handle.job_id();
    eprintln!("  prove submitted job_id={:?}, sleeping {}s …", job_id, CANCEL_DELAY.as_secs());
    tokio::time::sleep(CANCEL_DELAY).await;

    eprintln!("  cancelling …");
    let cancelled = handle.cancel().await?;
    eprintln!("  cancel() returned: {cancelled}");

    match handle.await {
        Ok(_) => anyhow::bail!("prove handle resolved Ok after cancel — cancel did not propagate"),
        Err(e) => eprintln!("  prove handle resolved with error (expected): {e}"),
    }
    Ok(())
}

/// Submit a long-running prove and cancel only after the worker enters the
/// Prove phase. We listen for `JobEvent::Progress(PROGRESS_PROVE)` instead of
/// timing it: the SDK already pipes phase transitions through this event, so
/// the cancel lands deterministically inside Prove regardless of how long
/// Contributions took on the box.
async fn run_prove_with_cancel_in_prove_phase(client: &RemoteClient) -> Result<()> {
    let input = ZiskStream::unix();
    let prove_started = Arc::new(Notify::new());
    let notify = prove_started.clone();

    let mut handle = client
        .prove(&ELF_LIVENESS, input.clone())
        .executor(ExecutorKind::Assembly)
        .on(JobEvent::Progress(PROGRESS_PROVE), move |_| notify.notify_one())
        .run()?;
    input.write(&1u64); // mode = long
    input.write(&7u64); // value (seed for the busy-loop)
    input.flush()?;

    let job_id = handle.job_id();
    eprintln!("  prove submitted job_id={job_id:?}, waiting for Prove phase …");
    prove_started.notified().await;

    eprintln!("  Prove phase entered, cancelling …");
    let cancelled = handle.cancel().await?;
    eprintln!("  cancel() returned: {cancelled}");

    match handle.await {
        Ok(_) => anyhow::bail!("prove handle resolved Ok after cancel — cancel did not propagate"),
        Err(e) => eprintln!("  prove handle resolved with error (expected): {e}"),
    }
    Ok(())
}

/// Execute the `panic_modes` guest with input=0 — the guest hits an explicit
/// `panic!()`. The worker reports the task failure and recovery kicks in,
/// just like the truncated-input scenario, but here the failure originates
/// in the guest's own code rather than from the host's stream.
async fn run_panic_execute(client: &RemoteClient) -> Result<()> {
    let stdin = ZiskStdin::new();
    stdin.write(&0u64); // input that triggers guest's `panic!()`
    let handle = client.execute(&ELF_PANIC_MODES, stdin).executor(ExecutorKind::Assembly).run()?;
    match handle.await {
        Ok(_) => anyhow::bail!("panic_modes execute resolved Ok with input that should panic"),
        Err(e) => eprintln!("  panic_modes execute resolved with error (expected): {e}"),
    }
    Ok(())
}

/// Execute with an *incomplete stream*: writes only the first record (mode)
/// and flushes. The guest expects a second record (value); when it tries to
/// read past the available input the worker reports the task failure and
/// recovery kicks in.
async fn run_invalid_execute(client: &RemoteClient) -> Result<()> {
    let input = ZiskStream::unix();
    let handle =
        client.execute(&ELF_LIVENESS, input.clone()).executor(ExecutorKind::Assembly).run()?;
    input.write(&0u64); // mode (well-formed) — first record only
                        // Intentionally skip the second record and flush. The guest's second
                        // `ziskos::io::read::<u64>()` will run past the end of the input.
    input.flush()?;

    match handle.await {
        Ok(_) => anyhow::bail!("execute handle resolved Ok with truncated input"),
        Err(e) => eprintln!("  execute resolved with error (expected): {e}"),
    }
    Ok(())
}
