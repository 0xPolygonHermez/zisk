//! Two ASM executions in one process, each reading its own input.
//!
//! An ASM execution leaves its shared memory dirty: the ROM histogram is read after
//! the execution phase returns, so the input shmem cannot be rewound at the end of it
//! — rewinding drains the semaphores the histogram child is waiting on. Retiring it is
//! the job boundary's work instead, and each client that runs more than one job in a
//! process owns one. This is the only test that runs two jobs back to back in a single
//! process, which is the only way to observe that retirement: the pieces live in four
//! crates and the shared memory is real.
//!
//! What it asserts, and what it does not:
//!
//! * **asserts** that the second execution reads its *own* input. `fib_mod` loops `n`
//!   times and commits `(n, module, fib(n) mod module)`, so two values of `n` give both
//!   a different step count and different public outputs; a second execution that ran
//!   the first one's input would match it on both. This is why the inputs cannot be the
//!   same — the same input twice is indistinguishable from stale shared memory.
//! * **does not assert** the ordering between the histogram runner and the reset.
//!   Rewinding before the runner has been retired strands the assembly child on a
//!   semaphore nobody will post, which surfaces as a hang, not a failed assertion. That
//!   ordering is unit-tested where it lives, in `zisk_common::LateValue` and
//!   `zisk_sm_rom`; what only this test covers is the composite.
//!
//! Nothing here resets anything explicitly, deliberately: the client is supposed to do
//! that at its own job boundary, and this test fails if it does not.
//!
//! Witness mode is what makes it worth testing — `verify_constraints` builds the ROM
//! instance and computes its witness, which is where the histogram is read. An
//! execute-only client has no ROM state machine, so no histogram runner is ever spawned
//! and there is nothing to retire.
//!
//! The proving machinery needs far more stack than a test binary gives it by default —
//! every binary that drives proofman raises it at startup, and a `cargo test` binary
//! raises nothing. See `PROVING_STACK` for which threads have to be told.
//!
//! Linux-only — the asm executor depends on mmap/jit support not available on
//! macOS/Windows. Ignored by default: witness mode needs a generated proving key on
//! disk and starting the ASM microservices takes several seconds. Run it with:
//!
//!     cargo test -p integration-tests --test asm_job_boundary -- --ignored --nocapture

#![cfg(target_os = "linux")]

use std::path::PathBuf;

use zisk_sdk::{
    EmbeddedClient, EmbeddedClientBuilder, ExecutorKind, VerifyConstraintsExtension, ZiskStdin,
};
use zisk_test_artifacts::ELF_FIB_MOD;

/// Stack size for the two pools that run the proof: rayon's workers, which do the
/// witness computation, and tokio's blocking pool, where the SDK runs an embedded job.
/// Both default to 2 MiB and both overflow on it.
///
/// 64 MiB is what every binary that drives this machinery already gives it —
/// `proofman_setup.rs`, `worker_node.rs`, `recurser.rs` here, and pil2-proofman's own
/// CLI. A test binary is the one caller that has to say so itself.
const PROVING_STACK: usize = 64 * 1024 * 1024;

/// The two inputs must differ, or a stale-shmem reuse would look like a pass.
const FIRST_N: u32 = 1_000;
const SECOND_N: u32 = 5_000;
const MODULE: u32 = 233;

/// `fib_mod` reads `n` then `module`, and commits both back along with the result.
fn stdin_for(n: u32) -> ZiskStdin {
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    stdin.write(&MODULE);
    stdin
}

/// One ASM verify-constraints run, returning what the guest actually executed:
/// its step count and the public values it committed.
async fn run_once(client: &EmbeddedClient, n: u32) -> (u64, Vec<u64>) {
    let result = client
        .verify_constraints(&ELF_FIB_MOD, stdin_for(n))
        .executor(ExecutorKind::Assembly)
        .run()
        .expect("failed to submit verify_constraints")
        .await
        .unwrap_or_else(|e| panic!("verify_constraints failed for n={n}: {e}"));

    (result.get_execution_steps(), result.get_publics().public_u64())
}

#[test]
#[ignore = "requires a generated proving key and the ASM microservices; run with --ignored"]
fn two_asm_executions_in_one_process() {
    // Before anything touches the pools. `ok()` because a global pool can only be built
    // once and another test in this binary may have got there first.
    rayon::ThreadPoolBuilder::new().stack_size(PROVING_STACK).build_global().ok();

    tokio::runtime::Builder::new_current_thread()
        .thread_stack_size(PROVING_STACK)
        .enable_all()
        .build()
        .expect("failed to build the tokio runtime")
        .block_on(two_jobs_one_process());
}

async fn two_jobs_one_process() {
    let mut builder = EmbeddedClientBuilder::default().assembly();

    if let Some(pk) = std::env::var_os("ZISK_TEST_PROVING_KEY").map(PathBuf::from) {
        eprintln!("[asm_job_boundary] using ZISK_TEST_PROVING_KEY={}", pk.display());
        builder = builder.proving_key(pk);
    }

    let client = builder.build().expect("failed to build EmbeddedClient");

    client
        .setup(&ELF_FIB_MOD)
        .run()
        .expect("failed to submit setup")
        .await
        .expect("ROM setup failed");

    let (first_steps, first_publics) = run_once(&client, FIRST_N).await;
    let (second_steps, second_publics) = run_once(&client, SECOND_N).await;

    assert_ne!(
        first_steps, second_steps,
        "the second execution ran the same number of steps as the first — it reused the first job's input"
    );
    assert_ne!(
        first_publics, second_publics,
        "the second execution committed the first one's public values — it reused the first job's input"
    );
}
