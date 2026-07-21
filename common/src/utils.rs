use std::mem::MaybeUninit;

use crate::error::{CommonError, Result};

/// Marker for types whose all-zero bit pattern is a valid, fully-initialized value and
/// that carry no `Drop` glue — so a bulk-zeroed allocation is a valid `Vec<Self>`.
///
/// # Safety
///
/// Implementors must be valid when zero-initialized: no niche, no reference, no field
/// that forbids the all-zero pattern, and no `Drop` implementation.
pub(crate) unsafe trait Zeroable {}

// SAFETY: `AtomicU64` wraps a `u64` whose all-zero pattern is the valid value `0`, and it
// has no `Drop` glue.
unsafe impl Zeroable for std::sync::atomic::AtomicU64 {}

/// Marker for types with no invalid bit patterns, so an arbitrary byte buffer can be
/// reinterpreted into them without validation.
///
/// A private mirror of this trait lives in `zisk-stream` (`zisk_stream.rs`), which cannot
/// depend on `zisk-common`; keep the two in sync.
///
/// # Safety
///
/// Implementors must accept *every* bit pattern as a valid value. Integer types qualify;
/// `bool`, `char`, `NonZero*`, niche enums, and references do not.
pub(crate) unsafe trait AnyBitPattern {}

// SAFETY: `u8` has no invalid bit patterns.
unsafe impl AnyBitPattern for u8 {}
// SAFETY: `u64` has no invalid bit patterns.
unsafe impl AnyBitPattern for u64 {}

/// Creates a `Vec<DT>` of `size` elements by zeroing the backing allocation in bulk
/// instead of constructing each element — a fast path for atomic integer element types.
///
/// The `DT: Zeroable` bound guarantees the zeroed bytes form valid `DT` values, so this
/// function is safe.
// `Zeroable` is a crate-internal soundness marker, deliberately not part of the public API.
#[allow(private_bounds)]
pub fn create_atomic_vec<DT: Zeroable>(size: usize) -> Vec<DT> {
    let mut vec: Vec<MaybeUninit<DT>> = Vec::with_capacity(size);

    // SAFETY: `vec` has capacity for `size` elements, so the `size * size_of::<DT>()`
    // bytes zeroed by `write_bytes` lie within the allocation. `DT: Zeroable` makes the
    // all-zero pattern a valid `DT`, so afterwards those `size` elements are initialized
    // and `set_len(size)` is sound. `MaybeUninit<DT>` shares `DT`'s layout, so
    // transmuting `Vec<MaybeUninit<DT>>` to `Vec<DT>` preserves the allocation.
    unsafe {
        let ptr = vec.as_mut_ptr() as *mut u8;
        std::ptr::write_bytes(ptr, 0, size * std::mem::size_of::<DT>()); // Fast zeroing

        vec.set_len(size);
        std::mem::transmute(vec) // Convert Vec<MaybeUninit<DT>> -> Vec<DT>
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
/// The `U: AnyBitPattern` bound guarantees the reinterpreted bytes form valid `U`
/// values, so this function is safe.
///
/// # Arguments
/// * `v` - The source vector to reinterpret.
///
/// # Type Parameters
/// * `T` - Source element type; must be `Copy` (destructor-free and bitwise-copyable),
///   so drop behavior is identical on the zero-copy and copy paths.
/// * `U` - Destination element type
///
/// # Errors
///
/// - [`CommonError::Invalid`] if `U` is a zero-sized type.
// `AnyBitPattern` is a crate-internal soundness marker, deliberately not part of the public API.
#[allow(private_bounds)]
pub fn reinterpret_vec<T: Copy, U: AnyBitPattern>(v: Vec<T>) -> Result<Vec<U>> {
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
