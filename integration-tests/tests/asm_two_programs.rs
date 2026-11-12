//! Two programs set up on one ASM client in one process, with jobs alternating between them.
//!
//! The assembly services of every program set up in a process share one set of shared-memory
//! segments: the first program's setup creates them, and each later one opens them. That holds
//! only if every segment is large enough for every program. All are fixed-size except the ROM
//! histogram's output, whose size grows with the program's ROM. So the histogram segment must
//! be sized for the largest program a process may hold, not for whichever was set up first —
//! otherwise a larger program's histogram service writes past the end of the segment and dies
//! with SIGBUS, and the parent's histogram reader, mapped at the first program's size, rejects
//! the larger histogram.
//!
//! So the small program is set up first and run first: that is the order that sizes the segment
//! and the reader for it. `big_rom`'s ROM is large enough that its histogram needs more than the
//! small one's, rounded up to the segment granularity (see its source).
//!
//! Witness mode is what reads the histogram: `verify_constraints` builds the ROM instance's
//! witness from it. An execute-only run never asks for it, and would pass whatever its size.
//!
//! Each job is checked against the Rust emulator, which shares nothing with the assembly
//! services, so a histogram or a memory image left by the other program cannot pass unnoticed.
//!
//! Linux-only, for the same reason as `asm_job_boundary`. Ignored by default: witness mode needs
//! a generated proving key on disk, and setting up `big_rom`'s assembly takes about a minute.
//! Run it with:
//!
//!     cargo test -p integration-tests --test asm_two_programs -- --ignored --nocapture

#![cfg(target_os = "linux")]

use std::path::PathBuf;

use zisk_sdk::{
    EmbeddedClient, EmbeddedClientBuilder, ExecutorKind, GuestProgram, VerifyConstraintsExtension,
    ZiskStdin,
};
use zisk_test_artifacts::{ELF_BIG_ROM, ELF_FIB_MOD};

/// Stack size for rayon's workers; see `asm_job_boundary` for why a test binary must set it.
const PROVING_STACK: usize = 64 * 1024 * 1024;

const FIB_N: u32 = 1_000;
const FIB_MODULE: u32 = 233;
const BIG_ROM_SEED: u64 = 0x5eed;

fn fib_mod_input() -> ZiskStdin {
    let stdin = ZiskStdin::new();
    stdin.write(&FIB_N);
    stdin.write(&FIB_MODULE);
    stdin
}

fn big_rom_input() -> ZiskStdin {
    let stdin = ZiskStdin::new();
    stdin.write(&BIG_ROM_SEED);
    stdin
}

/// One verify-constraints run of `program` on `executor`, returning the public values it
/// committed.
async fn run_once(
    client: &EmbeddedClient,
    program: &GuestProgram,
    name: &str,
    stdin: ZiskStdin,
    executor: ExecutorKind,
) -> Vec<u64> {
    client
        .verify_constraints(program, stdin)
        .executor(executor)
        .run()
        .expect("failed to submit verify_constraints")
        .await
        .unwrap_or_else(|e| panic!("verify_constraints of {name} on {executor:?} failed: {e}"))
        .get_publics()
        .public_u64()
}

#[tokio::test]
#[ignore = "requires a generated proving key and the ASM microservices; run with --ignored"]
async fn two_programs_of_different_rom_size_in_one_process() {
    // Before anything reaches the pool, which rayon builds on first use. `ok()` because a
    // global pool can only be built once.
    rayon::ThreadPoolBuilder::new().stack_size(PROVING_STACK).build_global().ok();

    let mut builder = EmbeddedClientBuilder::default().assembly();
    if let Some(pk) = std::env::var_os("ZISK_TEST_PROVING_KEY").map(PathBuf::from) {
        eprintln!("[asm_two_programs] using ZISK_TEST_PROVING_KEY={}", pk.display());
        builder = builder.proving_key(pk);
    }
    let client = builder.build().expect("failed to build EmbeddedClient");

    // The small program first: its setup creates the shared segments.
    for program in [&ELF_FIB_MOD, &ELF_BIG_ROM] {
        client
            .setup(program)
            .run()
            .expect("failed to submit setup")
            .await
            .expect("ROM setup failed");
    }

    // Small, large, small: the switch in both directions, the small program's histogram read
    // first.
    let fib =
        run_once(&client, &ELF_FIB_MOD, "fib_mod", fib_mod_input(), ExecutorKind::Assembly).await;
    let big =
        run_once(&client, &ELF_BIG_ROM, "big_rom", big_rom_input(), ExecutorKind::Assembly).await;
    let fib_again =
        run_once(&client, &ELF_FIB_MOD, "fib_mod", fib_mod_input(), ExecutorKind::Assembly).await;

    let big_emulated =
        run_once(&client, &ELF_BIG_ROM, "big_rom", big_rom_input(), ExecutorKind::Emulator).await;
    let fib_emulated =
        run_once(&client, &ELF_FIB_MOD, "fib_mod", fib_mod_input(), ExecutorKind::Emulator).await;

    assert_eq!(
        big, big_emulated,
        "big_rom committed other public values on assembly than on the emulator"
    );
    assert_eq!(
        fib, fib_emulated,
        "fib_mod committed other public values on assembly than on the emulator"
    );
    assert_eq!(
        fib_again, fib,
        "fib_mod committed other public values after big_rom ran in between"
    );
}
