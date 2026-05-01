//! Shared data structures for complex field elements used by Fp2 syscalls.

/// A complex field element over a 256-bit base field, represented as `x + y·i`.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallComplex256 {
    pub x: [u64; 4],
    pub y: [u64; 4],
}

/// A complex field element over a 384-bit base field, represented as `x + y·i`.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallComplex384 {
    pub x: [u64; 6],
    pub y: [u64; 6],
}
