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

/// Reinterprets a `Vec<T>` as a `Vec<U>` by transmuting the underlying memory.
///
/// This function converts between vector types by reinterpreting the raw memory,
/// adjusting length and capacity based on the size ratio between types.
/// It performs internal unsafe operations but validates all safety requirements
/// before the conversion.
///
/// # Arguments
/// * `v` - The source vector to reinterpret.
///
/// # Returns
/// * `Ok(Vec<U>)` - A new vector that owns the same memory as the input vector
/// * `Err` - If validation fails (size incompatibility or alignment issues)
///
/// # Type Parameters
/// * `T` - Source element type
/// * `U` - Destination element type
pub fn reinterpret_vec<T: Default + Clone, U>(mut v: Vec<T>) -> anyhow::Result<Vec<U>> {
    let size_t = std::mem::size_of::<T>();
    let size_u = std::mem::size_of::<U>();

    // Total bytes in Vec<T>
    let total_bytes = v.len() * size_t;

    // Compute remainder to see if we need padding
    let rem = total_bytes % size_u;

    // If remainder exists, pad with zeroed T elements
    if rem != 0 {
        // Number of extra bytes needed
        let pad_bytes = size_u - rem;

        // Number of T elements to pad (round up)
        let pad_t = pad_bytes.div_ceil(size_t);

        v.extend(std::iter::repeat(T::default()).take(pad_t));
    }

    // Check that the pointer is properly aligned for U
    if v.as_ptr() as usize % std::mem::align_of::<U>() != 0 {
        return Err(anyhow::anyhow!(
            "Vec<{}> is not properly aligned for Vec<{}> (requires {}-byte alignment)",
            std::any::type_name::<T>(),
            std::any::type_name::<U>(),
            std::mem::align_of::<U>()
        ));
    }

    let len = (v.len() * size_t) / size_u;
    let cap = (v.capacity() * size_t) / size_u;
    let ptr = v.as_ptr() as *mut U;

    std::mem::forget(v);
    Ok(unsafe { Vec::from_raw_parts(ptr, len, cap) })
}
