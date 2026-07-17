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
///
/// # Safety / caller obligation
///
/// Although this function is not marked `unsafe`, the returned `Vec<U>` will be
/// deallocated using `U`'s layout. For that to match the layout the buffer was
/// allocated with, the caller must only use type pairs where
/// `align_of::<U>() == align_of::<T>()` and the byte capacity divides evenly by
/// `size_of::<U>()`. The runtime alignment check below only validates the pointer
/// value, **not** the allocation layout, so a mismatched alignment (e.g. `u8` → `u64`)
/// is undefined behaviour even when the `Ok` branch is taken.
///
/// # Errors
///
/// - [`CommonError::Invalid`] if `U` is a zero-sized type (the reinterpretation is
///   undefined for a ZST destination).
/// - [`CommonError::Invalid`] if the resulting pointer is not properly aligned for `U`.
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
    // SAFETY: `ptr` comes from `v`, which was allocated by the global allocator and is
    // forgotten just above so its buffer is not freed twice. `len`/`cap` are recomputed
    // for `U` from the byte size of the original allocation, and the padding loop ensures
    // the byte length is an exact multiple of `size_of::<U>()`, so no partial element is
    // exposed. The alignment check above guarantees `ptr` is suitably aligned for `U`.
    // The remaining layout precondition (equal alignment of `T`/`U` for the eventual
    // deallocation) is the caller's obligation — see this function's "caller obligation" docs.
    Ok(unsafe { Vec::from_raw_parts(ptr, len, cap) })
}
