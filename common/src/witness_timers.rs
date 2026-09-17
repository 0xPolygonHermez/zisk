//! Phase stopwatches for witness computations, compiled out unless the calling crate's
//! `witness_timers` feature is on.
//!
//! These are for the inside of a witness computation: how its own phases divide up (gathering the
//! inputs, filling the trace, merging per-batch multiplicity caches, flushing them to the `std`),
//! which is the granularity the `stats` command's per-air totals cannot show. They report at `info`
//! so a run with the feature on needs no `-v`, and each state machine emits one line per instance.
//!
//! Being macros rather than functions is what lets them vanish: `#[cfg]` inside a `macro_rules!`
//! body is evaluated where the macro is expanded, so each crate decides with its own
//! `witness_timers` feature and pays nothing when it is off — no `Instant::now()`, no formatting,
//! and no unused bindings to silence.
//!
//! ```ignore
//! phase_start!(t_fill);
//! ...                       // the phase
//! phase_end!(d_fill, t_fill);
//! ...
//! phase_log!("Mem[{}] fill {:.0}ms", segment_id, phase_ms!(d_fill));
//! ```
//!
//! # Why a phase is closed where it ends
//!
//! `phase_end!` captures the duration at the point the phase finishes, and `phase_ms!` formats that
//! captured value. Reading the stopwatch at log time instead would report the time from the phase's
//! *start* to the log, which for anything but the last phase is the rest of the function as well --
//! a mistake worth naming, because the resulting numbers look plausible: every phase comes out
//! roughly equal to the one after it.
//!
//! # Caveat
//!
//! A `phase_ms!` outside a `phase_log!` will not compile with the feature off, because the binding
//! it reads does not exist. Read a duration only from inside a `phase_log!`.

/// Starts a stopwatch named `$name`. Expands to nothing without `witness_timers`.
#[macro_export]
macro_rules! phase_start {
    ($name:ident) => {
        #[cfg(feature = "witness_timers")]
        let $name = std::time::Instant::now();
    };
}

/// Closes the phase started as `$since`, capturing its duration as `$name`. Expands to nothing
/// without `witness_timers`.
#[macro_export]
macro_rules! phase_end {
    ($name:ident, $since:ident) => {
        #[cfg(feature = "witness_timers")]
        let $name = $since.elapsed();
    };
}

/// A duration captured by [`phase_end!`], in milliseconds. Only valid inside a [`phase_log!`],
/// which is the only place that survives without the feature.
#[macro_export]
macro_rules! phase_ms {
    ($name:ident) => {
        $name.as_secs_f64() * 1e3
    };
}

/// Logs a phase breakdown at `info`. Expands to nothing without `witness_timers`, which is what
/// makes the `phase_ms!` calls in its arguments disappear along with it.
#[macro_export]
macro_rules! phase_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "witness_timers")]
        tracing::info!($($arg)*);
    };
}

/// A stopwatch shared by parallel batches, reporting the slowest of them.
///
/// The batches of a fill run under `rayon`, so each has to record its own timing somewhere shared;
/// this keeps the maximum. Expands to nothing without `witness_timers`.
#[macro_export]
macro_rules! phase_max_start {
    ($name:ident) => {
        #[cfg(feature = "witness_timers")]
        let $name = std::sync::atomic::AtomicU64::new(0);
    };
}

/// Records `$since`'s elapsed time into the shared maximum `$name`.
#[macro_export]
macro_rules! phase_max_record {
    ($name:ident, $since:ident) => {
        #[cfg(feature = "witness_timers")]
        $name.fetch_max($since.elapsed().as_micros() as u64, std::sync::atomic::Ordering::Relaxed);
    };
}

/// The slowest time recorded in `$name`, in milliseconds. Only valid inside a [`phase_log!`].
#[macro_export]
macro_rules! phase_max_ms {
    ($name:ident) => {
        $name.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e3
    };
}
