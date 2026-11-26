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
/// `_RJEM_MALLOC_CONF` environment variable so settings can be swept without
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
///
/// tikv-jemalloc-sys builds jemalloc with its symbols prefixed by `_rjem_`
/// (we do not enable its `unprefixed_malloc_on_supported_platforms` feature),
/// so jemalloc reads `_rjem_malloc_conf`, as a C `const char *`. Exported as
/// plain `malloc_conf`, or as a Rust slice, it is never read.
#[cfg(feature = "jemalloc")]
#[allow(non_upper_case_globals)]
#[export_name = "_rjem_malloc_conf"]
pub static MALLOC_CONF: MallocConf = MallocConf(c"background_thread:true".as_ptr());

/// A C string pointer jemalloc reads at start-up.
#[cfg(feature = "jemalloc")]
#[repr(transparent)]
pub struct MallocConf(*const std::os::raw::c_char);

// SAFETY: it points to a `'static` C string literal, never written.
#[cfg(feature = "jemalloc")]
unsafe impl Sync for MallocConf {}

#[cfg(all(test, feature = "jemalloc"))]
mod tests {
    /// jemalloc reads [`super::MALLOC_CONF`]: the option it sets is on.
    #[test]
    fn jemalloc_reads_the_compile_time_configuration() {
        // The environment overrides the compile-time configuration: when it sets this option,
        // jemalloc reports the override, which says nothing about what was exported.
        if std::env::var_os("_RJEM_MALLOC_CONF")
            .is_some_and(|conf| conf.to_string_lossy().contains("background_thread"))
        {
            return;
        }

        let mut on = false;
        let mut len = std::mem::size_of::<bool>();
        // SAFETY: `opt.background_thread` is a `bool` the size `len` says.
        let rc = unsafe {
            tikv_jemalloc_sys::mallctl(
                c"opt.background_thread".as_ptr(),
                &mut on as *mut bool as *mut std::os::raw::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!(rc, 0);
        assert!(on, "jemalloc did not read the exported configuration");
    }
}
