//! Definition of the `SyscallPoint256` structure, representing a point with two 256-bit coordinates.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallPoint256 {
    pub x: [u64; 4],
    pub y: [u64; 4],
}
