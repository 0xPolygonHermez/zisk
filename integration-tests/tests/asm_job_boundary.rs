//! Two ASM executions in one process, each reading its own input.
//!
//! An ASM execution retires its shared memory at the end of the execution phase: the
//! ROM-histogram runner is joined, then the hints stream and the input shmem are rewound so
//! the next execution starts from a known state. This is the only test that runs two of them
//! back to back in one process, which is the only way to observe that retirement — the
//! pieces live in three crates and the shared memory is real.
//!
//! What it asserts, and what it does not:
//!
//! * **asserts** that the second execution reads its *own* input. `fib_mod` loops `n` times,
//!   so two values of `n` give two step counts that must differ; a second execution that ran
//!   the first one's input would match it. This is why the inputs cannot be the same — the
//!   same input twice is indistinguishable from a stale shmem.
//! * **does not assert** the runner-join ordering. Retiring the memory before the join
//!   strands the RH child on a semaphore nobody will post, which surfaces as a hang, not a
//!   failed assertion. The ordering itself is unit-tested where it lives, in
//!   `zisk_asm_runner::RhCell` and `zisk_sm_rom`; what only this test covers is the
//!   composite, in a real process with real shared memory.
//!
//! Witness mode is what makes this worth testing: an execute-only client has no ROM state
//! machine, so no ROM-histogram runner is ever spawned and there is nothing to retire.
//!
//! Linux-only — the asm executor depends on mmap/jit support not available on macOS/Windows.
//! Ignored by default: witness mode needs a generated proving key on disk, and starting the
//! ASM microservices takes several seconds. Run it with:
//!
//!     cargo test -p integration-tests --test asm_job_boundary -- --ignored --nocapture

#![cfg(target_os = "linux")]

use std::path::PathBuf;

use zisk_sdk::{EmbeddedClientBuilder, ExecutorKind, ZiskStdin};
use zisk_test_artifacts::ELF_FIB_MOD;

/// `fib_mod` reads `n` then `module`, and loops `n` times — so `n` sets the step count.
fn fib_mod_input(n: u32) -> ZiskStdin {
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    stdin.write(&233u32);
    stdin
}

#[tokio::test]
#[ignore = "needs a generated proving key and the ASM microservices"]
async fn two_asm_executions_in_one_process_each_read_their_own_input() {
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

    let first = client
        .execute(&ELF_FIB_MOD, fib_mod_input(1_000))
        .executor(ExecutorKind::Assembly)
        .run()
        .expect("failed to submit the first execute")
        .await
        .expect("first execution failed");

    // The first execution left its ROM-histogram runner joined and its shared memory
    // retired. The second has to start from that state — not from a stale one, and not from
    // one an in-flight child is still reading.
    let second = client
        .execute(&ELF_FIB_MOD, fib_mod_input(50_000))
        .executor(ExecutorKind::Assembly)
        .run()
        .expect("failed to submit the second execute")
        .await
        .expect("second execution must not hang, error, or read stale input");

    let (first_steps, second_steps) = (first.get_execution_steps(), second.get_execution_steps());
    assert!(first_steps > 0, "the first execution should run at least one step");
    assert!(
        second_steps > first_steps,
        "fib_mod(50_000) must take more steps than fib_mod(1_000); got {second_steps} and \
         {first_steps}, so the second execution did not read its own input — the ASM shared \
         memory was not retired at the job boundary"
    );
}
