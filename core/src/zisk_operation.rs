use crate::{InstContext, ZiskOperationType};

/// ZisK instruction, containing the opcode and a pointer to the function that implements it
#[derive(Debug, PartialEq, Clone)]
pub struct ZiskOperation {
    /// Operation name
    pub n: &'static str,
    /// Operation type
    pub t: &'static str,
    /// Operation steps
    pub s: u64,
    /// Operation code (1 byte)
    pub c: u8,
    /// Operation function f(a,b)->(c,flag), where a, b, and c are 32-bit represented as 64-bit
    /// (Goldilocks) and flag is either 0 or 1
    pub f: fn(&mut InstContext) -> (),
}

impl ZiskOperation {
    pub fn op_type(&self) -> ZiskOperationType {
        match self.t {
            "i" => ZiskOperationType::Internal,
            "a" => ZiskOperationType::Arith,
            "a32" => ZiskOperationType::Arith,
            "am32" => ZiskOperationType::Arith,
            "b" => ZiskOperationType::Binary,
            "be" => ZiskOperationType::Binary,
            "k" => ZiskOperationType::Keccak,
            _ => panic!("ZiskOperation::op_type() found invalid t={}", self.t),
        }
    }
}
