//! Definition of the `SyscallComplex256` structure, representing a complex field element over a 256-bit field.
#[derive(Debug)]
#[repr(C)]
pub struct SyscallComplex256 {
    pub x: [u64; 4],
    pub y: [u64; 4],
}
