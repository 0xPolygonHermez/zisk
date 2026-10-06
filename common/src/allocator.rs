//! The one `#[global_allocator]` for every ZisK binary.
//!
//! `#[global_allocator]` may be declared in a library and applies to whichever
//! binary links it, and only one may exist per binary — declaring it here
//! rather than in each `main.rs` is what keeps `cargo-zisk`, `cargo-zisk-dev`
//! and `zisk-worker` from conflicting.
//!
//! **Why replace glibc's allocator.** glibc serves the 128 KB-32 MB collector
//! buffers from per-thread arenas. A buffer freed by a thread on one arena is
//! invisible to threads bound to another, and an arena never shrinks, so the
//! capacity accumulates across jobs. Measured on a worker over 18 jobs: glibc's
//! arena grew +12.6 GB while the bytes actually in use stayed flat at ~750 MB —
//! **96% of the growth was memory already freed**. jemalloc instead returns
//! memory to the OS on a decay timer.

/// jemalloc, replacing the system allocator for every binary linking this crate.
#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL_ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// Compile-time jemalloc configuration, overridable at runtime by the
/// `MALLOC_CONF` environment variable so settings can be swept without
/// rebuilding.
///
/// Only `background_thread` is set, and only because its default (`false`) is
/// unhelpful here: it makes jemalloc purge dirty pages from a background thread
/// rather than only during allocator calls, and the worker is idle between jobs
/// — exactly when we want pages handed back.
///
/// `narenas` is deliberately **not** capped. That would transplant the glibc
/// `MALLOC_ARENA_MAX` workaround onto an allocator without glibc's per-thread
/// retention problem, and risks contention across the ~250 threads the witness
/// phase runs.
#[cfg(feature = "jemalloc")]
#[allow(non_upper_case_globals)]
#[export_name = "malloc_conf"]
pub static MALLOC_CONF: &[u8] = b"background_thread:true\0";
