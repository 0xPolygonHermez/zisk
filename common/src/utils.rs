use std::fs::OpenOptions;
use std::mem::MaybeUninit;
use tracing_subscriber::{filter::FilterFn, fmt, prelude::*, EnvFilter};

pub fn create_atomic_vec<DT>(size: usize) -> Vec<DT> {
    let mut vec: Vec<MaybeUninit<DT>> = Vec::with_capacity(size);

    unsafe {
        let ptr = vec.as_mut_ptr() as *mut u8;
        std::ptr::write_bytes(ptr, 0, size * std::mem::size_of::<DT>()); // Fast zeroing

        vec.set_len(size);
        std::mem::transmute(vec) // Convert MaybeUninit<Vec> -> Vec<AtomicU64>
    }
}

#[inline(always)]
pub fn uninit_array<const N: usize>() -> MaybeUninit<[u64; N]> {
    MaybeUninit::uninit()
}

#[macro_export]
macro_rules! trace_file {
    ($($arg:tt)*) => {
        tracing::trace!(target: "screen_and_file", $($arg)*);
    };
}

#[macro_export]
macro_rules! debug_file {
    ($($arg:tt)*) => {
        tracing::debug!(target: "screen_and_file", $($arg)*);
    };
}

#[macro_export]
macro_rules! info_file {
    ($($arg:tt)*) => {
        tracing::info!(target: "screen_and_file", $($arg)*);
    };
}

#[macro_export]
macro_rules! warn_file {
    ($($arg:tt)*) => {
        tracing::warn!(target: "screen_and_file", $($arg)*);
    };
}

#[macro_export]
macro_rules! error_file {
    ($($arg:tt)*) => {
        tracing::error!(target: "screen_and_file", $($arg)*);
    };
}

pub fn init_tracing(log_path: &str) {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_path)
        .expect("Failed to open log file");

    let file_layer = fmt::layer()
        .with_writer(file)
        .with_ansi(false) // no color in file
        .with_target(false)
        .with_filter(FilterFn::new(|meta| meta.target() == "screen_and_file"));

    let stdout_layer = fmt::layer().with_writer(std::io::stdout).with_ansi(true).with_target(false);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with(stdout_layer)
        .with(file_layer)
        .init();
}
