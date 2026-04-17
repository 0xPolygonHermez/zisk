//! Shared data structures for elliptic curve points used by curve syscalls.

/// An affine elliptic curve point with two 256-bit coordinates `(x, y)`.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallPoint256 {
    pub x: [u64; 4],
    pub y: [u64; 4],
}

/// An affine elliptic curve point with two 384-bit coordinates `(x, y)`.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallPoint384 {
    pub x: [u64; 6],
    pub y: [u64; 6],
}
