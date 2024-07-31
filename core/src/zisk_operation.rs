/// ZisK instruction, containing the opcode and a pointer to the function that implements it
#[derive(Debug, PartialEq, Clone)]
pub struct ZiskOperation {
    /// Operation name
    pub n: &'static str,
    /// Operation type
    pub t: &'static str,
    /// Operation code (1 byte)
    pub c: u8,
    /// Operation function f(a,b)->(c,flag), where a, b, and c are 32-bit represented as 64-bit
    /// (Goldilocks) and flag is either 0 or 1
    pub f: fn(a: u64, b: u64) -> (u64, bool),
}
