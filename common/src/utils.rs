use std::mem::MaybeUninit;
use std::sync::atomic::AtomicU64;

use crate::error::{CommonError, Result};

/// Marker for types that are sound to reinterpret raw bytes as, in either direction:
/// an arbitrary byte buffer can be read as them, and their own bytes can be read raw.
///
/// A private mirror of this trait lives in `zisk-stream` (`zisk_stream.rs`), which cannot
/// depend on `zisk-common`; keep the two in sync.
///
/// # Safety
///
/// Implementors must both:
/// - accept *every* bit pattern as a valid value (so an arbitrary byte buffer can be
///   reinterpreted into them — the destination requirement), and
/// - contain no padding or otherwise uninitialized bytes (so reading their own bytes is
///   never a read of uninitialized memory — the source requirement).
///
/// Integer types qualify; `bool`, `char`, `NonZero*`, niche enums, references, and structs
/// with padding do not.
pub unsafe trait AnyBitPattern {}

// SAFETY: `u8` has no invalid bit patterns.
unsafe impl AnyBitPattern for u8 {}
// SAFETY: `u64` has no invalid bit patterns.
unsafe impl AnyBitPattern for u64 {}

/// Creates a `Vec<AtomicU64>` of `size` elements by zeroing the backing allocation in
/// bulk instead of constructing each element — a fast path for atomic counters.
pub fn create_atomic_vec(size: usize) -> Vec<AtomicU64> {
    let mut vec: Vec<MaybeUninit<AtomicU64>> = Vec::with_capacity(size);

    // SAFETY: `vec` has capacity for `size` elements, so the `size * size_of::<AtomicU64>()`
    // bytes zeroed by `write_bytes` lie within the allocation. `AtomicU64`'s all-zero
    // pattern is the valid value `0` and it has no `Drop` glue, so afterwards those `size`
    // elements are initialized and `set_len(size)` is sound. `MaybeUninit<AtomicU64>` shares
    // `AtomicU64`'s layout, so transmuting `Vec<MaybeUninit<AtomicU64>>` to `Vec<AtomicU64>`
    // preserves the allocation.
    unsafe {
        let ptr = vec.as_mut_ptr() as *mut u8;
        std::ptr::write_bytes(ptr, 0, size * std::mem::size_of::<AtomicU64>()); // Fast zeroing

        vec.set_len(size);
        std::mem::transmute(vec) // Convert Vec<MaybeUninit<AtomicU64>> -> Vec<AtomicU64>
    }
}

/// Reinterprets a `Vec<T>` as a `Vec<U>` over the same bytes.
///
/// A private mirror of this function lives in `zisk-stream` (`zisk_stream.rs`), which
/// cannot depend on `zisk-common`; keep the two in sync.
///
/// When the source allocation can legally be handed to `Vec<U>`'s deallocator —
/// identical element alignment and a byte length/capacity that are whole numbers of `U`
/// — the buffer is reused in place (zero-copy). Otherwise (e.g. `u8` → `u64`, where the
/// alignment differs) the bytes are copied into a fresh `Vec<U>` allocated — and
/// therefore freed — under `U`'s own layout, so the result is sound to drop on any
/// global allocator.
///
/// If the byte length is not a whole number of `U`, the trailing partial `U` is
/// zero-padded. Callers streaming data in chunks must therefore cut on `size_of::<U>()`
/// boundaries, or the padding will shift every subsequent value.
///
/// The `T: AnyBitPattern` bound guarantees the source bytes are fully initialized (no
/// padding to read as uninitialized memory) and the `U: AnyBitPattern` bound guarantees
/// the reinterpreted bytes form valid `U` values, so this function is safe.
///
/// # Arguments
/// * `v` - The source vector to reinterpret.
///
/// # Type Parameters
/// * `T` - Source element type; must be `Copy` (destructor-free and bitwise-copyable, so
///   drop behavior is identical on the zero-copy and copy paths) and `AnyBitPattern` (no
///   padding/uninitialized bytes, so reading its raw bytes is sound).
/// * `U` - Destination element type; must be `AnyBitPattern`.
///
/// # Errors
///
/// - [`CommonError::Invalid`] if `U` is a zero-sized type.
pub fn reinterpret_vec<T: Copy + AnyBitPattern, U: AnyBitPattern>(v: Vec<T>) -> Result<Vec<U>> {
    let size_t = std::mem::size_of::<T>();
    let size_u = std::mem::size_of::<U>();

    // A zero-sized `U` would make the `% size_u` / `/ size_u` arithmetic below divide by
    // zero; reject it explicitly rather than panic.
    if size_u == 0 {
        return Err(CommonError::Invalid(format!(
            "cannot reinterpret Vec<{}> as Vec<{}>: destination type is zero-sized",
            std::any::type_name::<T>(),
            std::any::type_name::<U>()
        )));
    }

    let byte_len = v.len() * size_t;
    let byte_cap = v.capacity() * size_t;
    let len = byte_len.div_ceil(size_u); // whole `U` count, rounding a partial tail up

    // Zero-copy is sound only when the source allocation matches `Vec<U>`'s layout
    // exactly: identical alignment (so `ptr` is aligned for `U` and the free-time
    // `Layout` matches the allocation), a byte capacity that is a whole number of `U` (so
    // `cap` is not truncated), and a byte length already a whole number of `U` (a reused
    // buffer cannot grow to pad a partial tail).
    if std::mem::align_of::<T>() == std::mem::align_of::<U>()
        && byte_cap % size_u == 0
        && byte_len % size_u == 0
    {
        let cap = byte_cap / size_u;
        let ptr = v.as_ptr() as *mut U;
        std::mem::forget(v);
        // SAFETY: `ptr` comes from `v`'s allocation, forgotten just above so it is freed
        // exactly once — by the returned `Vec`. Equal alignment makes `ptr` aligned for
        // `U` and the layout identical under either type; `byte_cap % size_u == 0` makes
        // `cap * size_u == byte_cap`, so the returned `Vec` frees precisely the original
        // allocation, and `byte_len % size_u == 0` makes `len` cover it with no partial
        // element. `U: AnyBitPattern` makes every byte a valid `U`.
        return Ok(unsafe { Vec::from_raw_parts(ptr, len, cap) });
    }

    // Layouts differ (e.g. `u8` → `u64`): reusing the buffer would free it under a
    // different `Layout` than it was allocated with — undefined behaviour, and not merely
    // theoretical under jemalloc/mimalloc/Miri or a non-glibc libc. Copy into a fresh
    // `Vec<U>` owned end-to-end under `U`'s own layout, zero-filling any partial trailing
    // `U`. The source `v` is left untouched and dropped normally at end of scope.
    let mut out: Vec<U> = Vec::with_capacity(len);
    let out_bytes = len * size_u;
    // SAFETY: `out` reserves `out_bytes >= byte_len` bytes in a distinct allocation, so
    // copying the `byte_len` source bytes is in-bounds and non-overlapping; the remaining
    // `[byte_len, out_bytes)` tail is then zeroed, so all `out_bytes` bytes are
    // initialized and `set_len(len)` is sound. `U: AnyBitPattern` makes every byte
    // pattern (source data and zero padding alike) a valid `U`.
    unsafe {
        let dst = out.as_mut_ptr() as *mut u8;
        std::ptr::copy_nonoverlapping(v.as_ptr() as *const u8, dst, byte_len);
        std::ptr::write_bytes(dst.add(byte_len), 0, out_bytes - byte_len);
        out.set_len(len);
    }
    Ok(out)
}
