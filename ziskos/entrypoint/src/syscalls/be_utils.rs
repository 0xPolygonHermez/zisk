//! Helpers shared by the big-endian (`_be`) syscall variants.
//!
//! The `_be` precompiles expect/produce 256-bit and 384-bit values where both
//! the limb order and each limb's byte order are reversed with respect to the
//! native little-endian representation used by `precompiles_helpers`. These
//! helpers perform the conversion in both directions (the transformation is
//! its own inverse).

#![cfg(not(zisk_guest))]

#[inline(always)]
pub(super) fn be_swap_4(a: &[u64; 4]) -> [u64; 4] {
    [a[3].swap_bytes(), a[2].swap_bytes(), a[1].swap_bytes(), a[0].swap_bytes()]
}

#[inline(always)]
pub(super) fn be_swap_6(a: &[u64; 6]) -> [u64; 6] {
    [
        a[5].swap_bytes(),
        a[4].swap_bytes(),
        a[3].swap_bytes(),
        a[2].swap_bytes(),
        a[1].swap_bytes(),
        a[0].swap_bytes(),
    ]
}
