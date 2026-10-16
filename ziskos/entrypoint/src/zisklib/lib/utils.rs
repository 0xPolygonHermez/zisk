//! Comparison helpers, alignment checks, and byte-order conversion utilities
//! for `[u64]` slices and raw byte buffers.

#[cfg(zisk_guest)]
use crate::alloc_extern::vec::Vec;

/// Check if a pointer is 8-byte aligned.
#[inline(always)]
pub fn is_aligned_8(ptr: *const u8) -> bool {
    (ptr as usize) & 0x7 == 0
}

/// A value placed on an 8-byte boundary, for a local the precompiles read or write
/// as u64 words (their pointers must be 8-byte aligned) while its own type, e.g.
/// `[u32; N]` or `[u8; N]`, would only guarantee 4- or 1-byte alignment.
#[repr(C, align(8))]
pub(crate) struct Align8<T>(pub(crate) T);

/// The unaligned paths of the helpers below, kept out of line: inlined, their byte
/// loads and stores raise the register pressure of the whole function, and the
/// common aligned path then pays for extra callee-saved registers on every call.
///
/// # Safety
/// `src` must be valid for reading a `T`.
#[cold]
#[inline(never)]
pub(crate) unsafe fn read_unaligned_cold<T>(src: *const T) -> T {
    src.read_unaligned()
}

/// See [`read_unaligned_cold`].
///
/// # Safety
/// `dst` must be valid for writing a `T`.
#[cold]
#[inline(never)]
pub(crate) unsafe fn write_unaligned_cold<T>(dst: *mut T, value: T) {
    dst.write_unaligned(value)
}

/// Writes `value` to a C caller's buffer `dst`, which need not be aligned for `T`.
/// An aligned `dst` gets the plain typed store; an unaligned one, which a reference
/// or a typed store must never see, gets `write_unaligned`. The target has no fast
/// unaligned access, so that path is byte stores, and the test keeps it off the
/// common aligned path.
///
/// # Safety
/// `dst` must be valid for writing a `T`.
#[inline(always)]
pub(crate) unsafe fn write_maybe_unaligned<T>(dst: *mut T, value: T) {
    if (dst as usize) % core::mem::align_of::<T>() == 0 {
        dst.write(value);
    } else {
        write_unaligned_cold(dst, value);
    }
}

/// Reads a `T` from a C caller's buffer `src`, which need not be aligned for `T`
/// (the counterpart of [`write_maybe_unaligned`], for in/out buffers).
///
/// # Safety
/// `src` must be valid for reading a `T`.
#[inline(always)]
pub(crate) unsafe fn read_maybe_unaligned<T>(src: *const T) -> T {
    if (src as usize) % core::mem::align_of::<T>() == 0 {
        src.read()
    } else {
        read_unaligned_cold(src)
    }
}

/// `let_input!(name = ptr; [u64; N])` binds `name: &[u64; N]` to a C caller's input
/// at `ptr`, which need not be 8-byte aligned: in place when it is (a reference to
/// it is then sound, and the precompiles may take it), else to an aligned copy
/// that lives in the caller's scope. The test costs a couple of steps; copying
/// every input would cost far more.
///
/// Must be used in an `unsafe` context, with `ptr` valid for reading the type.
macro_rules! let_input {
    ($name:ident = $ptr:expr; $ty:ty) => {
        let copy: $ty;
        let ptr = $ptr as *const $ty;
        let $name: &$ty = if (ptr as usize) % core::mem::align_of::<$ty>() == 0 {
            &*ptr
        } else {
            copy = $crate::zisklib::read_unaligned_cold(ptr);
            &copy
        };
    };
}
pub(crate) use let_input;

/// The first statement of a C-ABI function whose u64 buffers may be unaligned:
/// `unaligned_redirect!([a_ptr: 4, ...], [result_ptr: 4, ...], |ins, outs| f(...),
/// |ins, outs| f(..., hints))`, listing each `*const u64` input and `*mut u64` output
/// with its length in u64 words (a constant, or an expression of the other
/// arguments), and the call to the function itself, without and with the `hints`
/// argument.
///
/// One test covers every pointer. When all are 8-byte aligned (the normal case) it
/// falls through to the body, which runs as written: its references, and the
/// precompiles they reach, are then sound. Otherwise [`ffi_unaligned`] calls the
/// function again on aligned copies of the inputs and outputs, and writes the
/// outputs back. The body stays the only caller of what it calls, so the compiler
/// inlines it as before.
///
/// Must be used in an `unsafe` context, with every pointer valid for its length.
macro_rules! unaligned_redirect {
    ([$($i:ident: $ni:expr),*], [$($o:ident: $no:expr),*],
     |$ins:ident, $outs:ident| $call:expr, |$hins:ident, $houts:ident| $call_hints:expr) => {
        if (0usize $(| $i as usize)* $(| $o as usize)*) & 0x7 != 0 {
            #[cfg(not(feature = "hints"))]
            return $crate::zisklib::ffi_unaligned(
                &[$(($i, $ni)),*],
                &[$(($o, $no)),*],
                &mut |$ins: &[*const u64], $outs: &[*mut u64]| $call,
            );
            #[cfg(feature = "hints")]
            return $crate::zisklib::ffi_unaligned(
                &[$(($i, $ni)),*],
                &[$(($o, $no)),*],
                &mut |$hins: &[*const u64], $houts: &[*mut u64]| $call_hints,
            );
        }
    };
}
pub(crate) use unaligned_redirect;

/// The unaligned path of [`unaligned_redirect!`]: copies each input `(ptr, len)` and
/// each output into aligned buffers, runs `run` on those, and copies the outputs back.
/// Kept out of line so that it adds nothing to the aligned path.
///
/// The copies are bytewise into `MaybeUninit<u64>` storage, never reads of `u64`
/// values: an output buffer need not be initialized (a C caller may pass fresh
/// storage, and some functions leave it untouched on failure), and its bytes are
/// carried over as they are. So whatever the call does not write comes back to the
/// caller exactly as it was, initialized or not.
///
/// # Safety
/// Every pointer must be valid for reading (and, for outputs, writing) its `len`
/// u64 words.
#[cold]
#[inline(never)]
pub(crate) unsafe fn ffi_unaligned<R>(
    ins: &[(*const u64, usize)],
    outs: &[(*mut u64, usize)],
    run: &mut dyn FnMut(&[*const u64], &[*mut u64]) -> R,
) -> R {
    use core::mem::MaybeUninit;
    // An aligned copy of the `n` u64 words at `p`, byte for byte. `copy_nonoverlapping`
    // is an untyped copy, so uninitialized bytes are fine, but it needs a non-null
    // pointer even for 0 bytes, and an empty buffer may be null (see `slice_from_ffi`):
    // nothing is copied then.
    let copy = |p: *const u64, n: usize| -> Vec<MaybeUninit<u64>> {
        let mut v: Vec<MaybeUninit<u64>> = Vec::with_capacity(n);
        if n > 0 {
            core::ptr::copy_nonoverlapping(p as *const u8, v.as_mut_ptr() as *mut u8, n * 8);
            v.set_len(n);
        }
        v
    };
    let in_copies: Vec<Vec<MaybeUninit<u64>>> = ins.iter().map(|&(p, n)| copy(p, n)).collect();
    let mut out_copies: Vec<Vec<MaybeUninit<u64>>> =
        outs.iter().map(|&(p, n)| copy(p, n)).collect();
    let in_ptrs: Vec<*const u64> = in_copies.iter().map(|v| v.as_ptr() as *const u64).collect();
    let out_ptrs: Vec<*mut u64> =
        out_copies.iter_mut().map(|v| v.as_mut_ptr() as *mut u64).collect();
    let result = run(&in_ptrs, &out_ptrs);
    for (&(p, n), v) in outs.iter().zip(&out_copies) {
        if n > 0 {
            core::ptr::copy_nonoverlapping(v.as_ptr() as *const u8, p as *mut u8, n * 8);
        }
    }
    result
}

/// `len` u64s from a C caller's buffer at `ptr`, which need not be 8-byte aligned:
/// the caller's memory when it is, else a copy in `copy`. Null or dangling is fine
/// when `len` is 0 (see [`slice_from_ffi`]).
///
/// # Safety
/// When `len > 0`, `ptr` must be valid for reading `len` u64s.
#[inline]
pub(crate) unsafe fn u64s_from_ffi(ptr: *const u64, len: usize, copy: &mut Vec<u64>) -> &[u64] {
    if len == 0 || (ptr as usize) & 0x7 == 0 {
        slice_from_ffi(ptr, len)
    } else {
        copy.clear();
        copy.extend((0..len).map(|i| ptr.add(i).read_unaligned()));
        copy
    }
}

/// Convert a big-endian byte array to an array of 4 u64 limbs (little-endian order).
#[inline]
pub fn be_bytes_to_u64_4(bytes: &[u8; 32]) -> [u64; 4] {
    [
        u64::from_be_bytes(bytes[24..32].try_into().unwrap()),
        u64::from_be_bytes(bytes[16..24].try_into().unwrap()),
        u64::from_be_bytes(bytes[8..16].try_into().unwrap()),
        u64::from_be_bytes(bytes[0..8].try_into().unwrap()),
    ]
}

/// Convert an array of 4 u64 limbs (little-endian order) to a big-endian byte array.
#[inline]
pub fn u64_4_to_be_bytes(limbs: &[u64; 4]) -> [u8; 32] {
    let b3 = limbs[3].to_be_bytes();
    let b2 = limbs[2].to_be_bytes();
    let b1 = limbs[1].to_be_bytes();
    let b0 = limbs[0].to_be_bytes();
    [
        b3[0], b3[1], b3[2], b3[3], b3[4], b3[5], b3[6], b3[7], b2[0], b2[1], b2[2], b2[3], b2[4],
        b2[5], b2[6], b2[7], b1[0], b1[1], b1[2], b1[3], b1[4], b1[5], b1[6], b1[7], b0[0], b0[1],
        b0[2], b0[3], b0[4], b0[5], b0[6], b0[7],
    ]
}

/// Convert a little-endian byte array to an array of 4 u64 limbs (little-endian order).
#[inline]
pub fn le_bytes_to_u64_4(bytes: &[u8; 32]) -> [u64; 4] {
    [
        u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
    ]
}

/// Convert an array of 4 u64 limbs (little-endian order) to a little-endian byte array.
#[inline]
pub fn u64_4_to_le_bytes(limbs: &[u64; 4]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (chunk, limb) in out.chunks_mut(8).zip(limbs) {
        chunk.copy_from_slice(&limb.to_le_bytes());
    }
    out
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x > y`; otherwise, false.
pub fn gt(x: &[u64], y: &[u64]) -> bool {
    debug_assert_eq!(x.len(), y.len(), "x and y must have the same length");

    for i in (0..x.len()).rev() {
        if x[i] > y[i] {
            return true;
        } else if x[i] < y[i] {
            return false;
        }
    }
    false
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x < y`; otherwise, false.    
pub fn lt(x: &[u64], y: &[u64]) -> bool {
    debug_assert_eq!(x.len(), y.len(), "x and y must have the same length");

    for i in (0..x.len()).rev() {
        if x[i] < y[i] {
            return true;
        } else if x[i] > y[i] {
            return false;
        }
    }
    false
}

/// Given two n-bit number `x` and `y`, compares them and returns true if `x == y`; otherwise, false.
pub fn eq(x: &[u64], y: &[u64]) -> bool {
    debug_assert_eq!(x.len(), y.len(), "x and y must have the same length");

    for i in 0..x.len() {
        if x[i] != y[i] {
            return false;
        }
    }
    true
}

/// Returns true if x == 0
pub fn is_zero(x: &[u64]) -> bool {
    x.iter().all(|&w| w == 0)
}

/// Returns true if x == 1
pub fn is_one(x: &[u64]) -> bool {
    match x.split_first() {
        Some((&first, rest)) => first == 1 && rest.iter().all(|&w| w == 0),
        None => false,
    }
}

/// Returns true if x == 2
pub fn is_two(x: &[u64]) -> bool {
    match x.split_first() {
        Some((&first, rest)) => first == 2 && rest.iter().all(|&w| w == 0),
        None => false,
    }
}

/// Returns true if x is a power of two
pub fn is_power_of_two(x: &[u64]) -> bool {
    // TODO: I could simply hint the bit and the position and check it
    // A multiple-word number is a power of two if it has exactly one bit set across all words
    let mut found_one = false;
    for &word in x {
        if word != 0 {
            if found_one || (word & (word - 1)) != 0 {
                return false;
            }
            found_one = true;
        }
    }
    found_one
}

/// Returns true if x fits in a single 64-bit word (i.e., x < 2^64).
pub fn is_short(x: &[u64]) -> bool {
    x.iter().skip(1).all(|&word| word == 0)
}

/// Builds a shared slice from an FFI pointer, tolerating a null or dangling pointer
/// when `len == 0`. `from_raw_parts` demands a valid, aligned pointer even for an
/// empty slice, which C callers do not reliably provide: an empty buffer commonly
/// decays to null on the C side.
///
/// # Safety
/// When `len > 0`, `ptr` must be valid and aligned for reads of `len` elements.
#[inline]
pub(crate) unsafe fn slice_from_ffi<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    if len == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(ptr, len)
    }
}
