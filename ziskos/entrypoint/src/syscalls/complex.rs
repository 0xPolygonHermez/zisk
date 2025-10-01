/// Definition of the `SyscallComplex256` structure, representing a complex field element over a 256-bit field.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallComplex256 {
    pub x: [u64; 4],
    pub y: [u64; 4],
}

/// Definition of the `SyscallComplex384` structure, representing a complex field element over a 384-bit field.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallComplex384 {
    pub x: [u64; 6],
    pub y: [u64; 6],
}
