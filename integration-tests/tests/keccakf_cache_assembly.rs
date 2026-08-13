//! Exercises the Keccak-f cache fcalls through the assembly executor, whose cache lives in
//! `lib-c` (`keccakf_cache.cpp`) rather than in the Rust emulator's instruction context.
//!
//! The guest asserts every hit, miss and registration lifetime itself, so a wrong index or a
//! missed registration surfaces as a failed execution. The emulator side of the same guest is
//! covered by `precomp-keccakf`'s `keccakf_cache_tests`.
//!
//! Linux-only — the asm executor depends on mmap/jit support not available on macOS/Windows.
//! On other platforms this file compiles to an empty test binary.
//!
//! Lives in its own integration test file because the SDK enforces a per-process
//! `ProverClient` singleton (`sdk/src/client.rs`).
//!
//! Run: cargo test -p integration-tests --test keccakf_cache_assembly -- --ignored --nocapture

#![cfg(target_os = "linux")]

use test_artifacts::ELF_KECCAKF_CACHE;
use zisk_sdk::{EmbeddedClientBuilder, ZiskStdin};

#[test]
#[ignore]
fn keccakf_cache_assembly() {
    let client = EmbeddedClientBuilder::default()
        .assembly()
        .execute_only()
        .build()
        .expect("failed to build EmbeddedExecuteOnlyClient");

    client.setup(&ELF_KECCAKF_CACHE, false).expect("ROM setup failed");

    client
        .execute(&ELF_KECCAKF_CACHE, ZiskStdin::new(), None)
        .expect("keccakf cache guest execution failed");
}
