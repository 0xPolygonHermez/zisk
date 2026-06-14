//! Per-call scratch arena for accelerator functions.
//!
//! A fixed 2 MiB static buffer (`SCRATCH_BUF`) lives in `.bss` (zero-
//! initialised, no heap involvement).  `BumpScratch::reset` rewinds the bump
//! pointer to the start of that buffer at the beginning of every accelerator
//! call, recycling all prior allocations in bulk.
//!
//! `init_scratch` is a thin delegating wrapper around `reset`; it exists only
//! to preserve the call-site convention in `_zisk_main`.

#[cfg(zisk_guest)]
use crate::ziskos_memcpy;
#[cfg(zisk_guest)]
use crate::ziskos_memset;
#[cfg(zisk_guest)]
use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr::NonNull,
};

#[allow(dead_code)]
/// Size of the per-call scratch arena (2 MiB).
pub const SCRATCH_SIZE: usize = 2 * 1024 * 1024;

/// Backing storage for the scratch arena.
///
/// Declared as `[u64; N]` so that the array has 8-byte alignment without
/// requiring a wrapper type.  Lives in `.bss` (zero-initialised at startup).
#[cfg(zisk_guest)]
static mut SCRATCH_BUF: [u64; SCRATCH_SIZE / 8] = [0u64; SCRATCH_SIZE / 8];

/// Current bump pointer into `SCRATCH_BUF`.
#[cfg(zisk_guest)]
static mut SCRATCH_POS: usize = 0;

/// Zero-sized type used as the allocator for scratch-backed `Vec`s.
///
/// On `zisk_guest` this implements `core::alloc::Allocator` (via
/// `allocator_api`) and routes all allocations into the scratch arena.
/// `dealloc` is a no-op; memory is reclaimed in bulk at each accelerator
/// entry by calling `BumpScratch::reset`.
///
/// On host builds the struct still exists so that call sites compile without
/// any `#[cfg]` guards; its `reset` method is a no-op.
#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct BumpScratch;

/// Reset the scratch arena to the start of the static backing buffer.
///
/// Delegates to `BumpScratch::reset`.  Exists to preserve the call-site
/// convention in `_zisk_main`; on non-guest builds this is a no-op.
#[allow(dead_code)]
pub unsafe fn init_scratch() {
    BumpScratch::reset();
}

impl BumpScratch {
    /// Rewind the arena to the start of `SCRATCH_BUF`, recycling all
    /// allocations since the last reset.  Must be the very first statement
    /// of every public accelerator entry point.
    ///
    /// On non-guest builds this is a no-op.
    #[inline(always)]
    #[allow(dead_code)]
    pub fn reset() {
        #[cfg(zisk_guest)]
        // SAFETY: single-threaded guest — no concurrent access.
        unsafe {
            SCRATCH_POS = core::ptr::addr_of_mut!(SCRATCH_BUF) as usize;
        }
    }
}

// ── Allocator impl (zisk_guest only) ─────────────────────────────────────────

#[cfg(zisk_guest)]
unsafe impl Allocator for BumpScratch {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // SAFETY: single-threaded guest.
        let pos = unsafe { SCRATCH_POS };
        let align = layout.align();
        // Layout guarantees align is a power of two and >= 1.
        debug_assert!(align.is_power_of_two());

        // Round up to the required alignment.
        // Use checked_add so that a pos near usize::MAX does not silently wrap
        // start below pos, bypassing the bounds check and returning address ~0.
        let start = pos.checked_add(align - 1).ok_or(AllocError)? & !(align - 1);

        let end = start.checked_add(layout.size()).ok_or(AllocError)?;

        // Compute the buffer's end address from the static's address.
        // No separate SCRATCH_TOP static needed; the compiler/linker resolves
        // addr_of_mut!(SCRATCH_BUF) to a link-time constant.
        let top = core::ptr::addr_of_mut!(SCRATCH_BUF) as usize + SCRATCH_SIZE;
        if end > top {
            // Scratch arena exhausted — SCRATCH_SIZE is too small for this call.
            return Err(AllocError);
        }
        unsafe { SCRATCH_POS = end };
        // SAFETY: `start` lies within the scratch region.
        let ptr = unsafe { NonNull::new_unchecked(start as *mut u8) };
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = self.allocate(layout)?;
        ziskos_memset!(ptr: ptr.as_ptr() as *mut u8, 0, layout.size());
        Ok(ptr)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        // Bulk-reset on the next accelerator entry; no per-dealloc cost.
    }

    #[inline(always)]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let new_ptr = self.allocate(new_layout)?;
        let dst = new_ptr.as_ptr() as *mut u8;
        let src = ptr.as_ptr();
        ziskos_memcpy!(ptr: dst, src, old_layout.size());
        Ok(new_ptr)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let new_ptr = self.grow(ptr, old_layout, new_layout)?;
        let extra_start = (new_ptr.as_ptr() as *mut u8).add(old_layout.size());
        let extra_len = new_layout.size() - old_layout.size();
        ziskos_memset!(ptr: extra_start, 0, extra_len);
        Ok(new_ptr)
    }

    #[inline(always)]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        _old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // Bump memory is never released; tail bytes of the old slot are simply
        // left unused until the next reset().  No allocation or copy needed.
        Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()))
    }
}

// ── ScratchVec and constructors ───────────────────────────────────────────────

/// A `Vec<T>` backed by the per-call scratch arena on `zisk_guest` targets,
/// or by the standard global allocator on host targets.
///
/// The full `Vec` API (push, len, deref-to-slice, …) is available in both
/// cases.
#[cfg(zisk_guest)]
pub type ScratchVec<T> = crate::alloc_crate::vec::Vec<T, BumpScratch>;

#[cfg(not(zisk_guest))]
pub type ScratchVec<T> = std::vec::Vec<T>;

/// Create a `ScratchVec<T>` pre-allocated for `capacity` elements (length = 0).
///
/// On `zisk_guest` this draws from the scratch arena.
/// On host targets this calls `Vec::with_capacity`.
#[inline(always)]
pub fn new_scratch_vec<T>(capacity: usize) -> ScratchVec<T> {
    #[cfg(zisk_guest)]
    {
        crate::alloc_crate::vec::Vec::with_capacity_in(capacity, BumpScratch)
    }
    #[cfg(not(zisk_guest))]
    {
        std::vec::Vec::with_capacity(capacity)
    }
}

/// Reports whether a value's bit pattern is all zeros.
///
/// Used by [`new_scratch_vec_filled_z`] to choose bulk-zero initialisation
/// (a single `memset`) over per-element cloning when the fill value is zero.
///
/// # Safety
/// `is_zero` must return `true` only when an all-zero bit pattern is a valid,
/// fully-initialised `T`.  A wrong implementation causes undefined behaviour in
/// [`new_scratch_vec_filled_z`], which is otherwise a safe function.
pub unsafe trait IsZero {
    fn is_zero(&self) -> bool;
}

unsafe impl IsZero for u64 {
    #[inline(always)]
    fn is_zero(&self) -> bool {
        *self == 0
    }
}

/// Create a `ScratchVec<T>` of `len` elements all set to `value`, with a zero
/// fast-path: when `value.is_zero()` is `true` (constant-folded by LLVM for
/// compile-time constants), the entire block is zeroed with a single
/// `ziskos_memset!` (guest) or `write_bytes` (host) before `set_len`,
/// avoiding per-element cloning.
#[inline(always)]
pub fn new_scratch_vec_filled_z<T: Clone + IsZero>(len: usize, value: T) -> ScratchVec<T> {
    let mut v = new_scratch_vec(len);
    if value.is_zero() {
        // SAFETY: T: IsZero guarantees all-zeros is a valid, fully-initialised T.
        #[cfg(zisk_guest)]
        ziskos_memset!(ptr: v.as_mut_ptr() as *mut u8, 0, len * core::mem::size_of::<T>());
        unsafe {
            #[cfg(not(zisk_guest))]
            core::ptr::write_bytes(v.as_mut_ptr(), 0, len);
            v.set_len(len);
        }
    } else {
        v.resize(len, value);
    }
    v
}

/// Create a `ScratchVec<T>` of `len` elements all initialised to `value`.
///
/// Equivalent to `vec![value; len]` but backed by the scratch arena on guest.
#[inline(always)]
pub fn new_scratch_vec_filled<T: Clone>(len: usize, value: T) -> ScratchVec<T> {
    let mut v = new_scratch_vec(len);
    v.resize(len, value);
    v
}

/// Copy a slice into a new `ScratchVec<T>`.
///
/// Equivalent to `slice.to_vec()` but backed by the scratch arena on guest.
#[inline(always)]
pub fn scratch_vec_from_slice<T: Copy>(slice: &[T]) -> ScratchVec<T> {
    let mut v = new_scratch_vec(slice.len());
    v.extend_from_slice(slice);
    v
}
