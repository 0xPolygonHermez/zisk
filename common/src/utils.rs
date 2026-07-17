use std::mem::MaybeUninit;

use crate::error::{CommonError, Result};

/// Creates a vector of `size` elements, zero-initialized and reinterpreted as `Vec<DT>`.
///
/// This is a fast path that zeroes the backing allocation in bulk instead of
/// constructing each element, intended for atomic integer element types.
///
/// # Safety / caller obligation
///
/// Although this function is not marked `unsafe`, it is only sound for a `DT` whose
/// all-zero bit pattern is a valid value — e.g. atomic integer types such as
/// [`AtomicU64`](std::sync::atomic::AtomicU64). Calling it with a `DT` that has no
/// valid all-zero representation (e.g. a type with a niche, or a non-trivial `Drop`)
/// is undefined behaviour.
pub fn create_atomic_vec<DT>(size: usize) -> Vec<DT> {
    let mut vec: Vec<MaybeUninit<DT>> = Vec::with_capacity(size);

    // SAFETY: `vec` has capacity for `size` elements, so the `size * size_of::<DT>()`
    // bytes written by `write_bytes` lie within the allocation. After zeroing, those
    // `size` elements are initialized, making `set_len(size)` sound. `MaybeUninit<DT>`
    // shares `DT`'s layout, so transmuting `Vec<MaybeUninit<DT>>` to `Vec<DT>` preserves
    // the allocation. The caller upholds that all-zero is a valid `DT` (see the
    // function's "caller obligation" docs).
    unsafe {
        let ptr = vec.as_mut_ptr() as *mut u8;
        std::ptr::write_bytes(ptr, 0, size * std::mem::size_of::<DT>()); // Fast zeroing

        vec.set_len(size);
        std::mem::transmute(vec) // Convert MaybeUninit<Vec> -> Vec<AtomicU64>
    }
}

/// Reinterprets a `Vec<T>` as a `Vec<U>` by reusing the source allocation in place
/// (zero-copy): the returned vector owns the same buffer, viewed as `U`.
///
/// The byte length is zero-padded up to a multiple of `size_of::<U>()` (with
/// `T::default()` elements) so the buffer divides evenly into `U` values.
///
/// # Arguments
/// * `v` - The source vector to reinterpret.
///
/// # Returns
/// * `Ok(Vec<U>)` - A vector that owns `v`'s buffer (zero-padded), viewed as `U`.
///
/// # Type Parameters
/// * `T` - Source element type
/// * `U` - Destination element type
///
/// # Preconditions (caller must uphold — this is zero-copy, nothing is validated)
///
/// 1. **Byte length should already be a multiple of `size_of::<U>()`.** When it is
///    not, the trailing partial `U` is silently zero-padded; for a chunked stream
///    (e.g. `u8` → `u64` processed chunk-by-chunk) that shifts every subsequent
///    value and corrupts the logical sequence. All current callers feed data whose
///    byte length is a multiple of 8 — `u64` payloads cut on 8-byte boundaries by
///    `ZiskStreamWriter` — so no padding ever occurs.
/// 2. **Every bit pattern of `U` must be valid.** Integer types such as `u64`
///    qualify; `bool`, niche enums, `NonZero*`, or `Drop` types do not — the bytes
///    are reinterpreted without validation.
/// 3. **Deallocation caveat.** The returned `Vec<U>` is freed under `U`'s layout
///    although the buffer was allocated under `T`'s. When
///    `align_of::<U>() != align_of::<T>()` (e.g. `u8` → `u64`) this violates the
///    global allocator's layout contract and is *technically* undefined behaviour;
///    it is sound in practice only because the system allocator (glibc `malloc`)
///    ignores the layout alignment on free. Do not rely on this under a strict
///    allocator (jemalloc/mimalloc/Miri) — reintroduce a copy for the
///    align-increasing case if that ever becomes the runtime.
///
/// # Errors
///
/// - [`CommonError::Invalid`] if `U` is a zero-sized type.
/// - [`CommonError::Invalid`] if the source pointer is not aligned for `U` (reads
///   would be unsound). The system allocator over-aligns, so this does not trigger
///   for the current callers.
pub fn reinterpret_vec<T: Default + Clone, U>(mut v: Vec<T>) -> Result<Vec<U>> {
    let size_t = std::mem::size_of::<T>();
    let size_u = std::mem::size_of::<U>();

    // A zero-sized `U` would make the `% size_u` / `/ size_u` arithmetic below divide
    // by zero; reject it explicitly rather than panic.
    if size_u == 0 {
        return Err(CommonError::Invalid(format!(
            "cannot reinterpret Vec<{}> as Vec<{}>: destination type is zero-sized",
            std::any::type_name::<T>(),
            std::any::type_name::<U>()
        )));
    }

    // Zero-pad so the byte length is an exact multiple of `size_u` (see precondition 1).
    let rem = (v.len() * size_t) % size_u;
    if rem != 0 {
        let pad_bytes = size_u - rem;
        let pad_t = pad_bytes.div_ceil(size_t);
        v.extend(std::iter::repeat(T::default()).take(pad_t));
    }

    // Reject a misaligned source rather than produce unaligned `U` reads.
    if v.as_ptr() as usize % std::mem::align_of::<U>() != 0 {
        return Err(CommonError::Invalid(format!(
            "Vec<{}> is not properly aligned for Vec<{}> (requires {}-byte alignment)",
            std::any::type_name::<T>(),
            std::any::type_name::<U>(),
            std::mem::align_of::<U>()
        )));
    }

    let len = (v.len() * size_t) / size_u;
    let cap = (v.capacity() * size_t) / size_u;
    let ptr = v.as_ptr() as *mut U;

    std::mem::forget(v);
    // SAFETY: `ptr` comes from `v`'s allocation, which is forgotten just above so the
    // buffer is freed exactly once (by the returned `Vec`). `len`/`cap` are the same
    // byte span recomputed in units of `U`, and the padding above makes the byte
    // length an exact multiple of `size_of::<U>()`, so no partial element is exposed.
    // The alignment check guarantees `ptr` is aligned for reads of `U`. The one
    // remaining obligation — that freeing under `U`'s layout matches the original
    // allocation — is the caller's (precondition 3 above).
    Ok(unsafe { Vec::from_raw_parts(ptr, len, cap) })
}
