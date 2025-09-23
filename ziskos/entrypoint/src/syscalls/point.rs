/// Definition of the `SyscallPoint256` structure, representing a point with two 256-bit coordinates.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallPoint256 {
    pub x: [u64; 4],
    pub y: [u64; 4],
}

/// Definition of the `SyscallPoint384` structure, representing a point with two 384-bit coordinates.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallPoint384 {
    pub x: [u64; 6],
    pub y: [u64; 6],
}
