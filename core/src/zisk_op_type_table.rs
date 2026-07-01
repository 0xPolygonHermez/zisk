// `OpType` and its `ZiskOperationType`/`Display`/`FromStr` mappings.
//
// `include!`d by `zisk_ops.rs` (item position) so it shares that module's scope
// (`ZiskOperationType`, `InvalidOpTypeError`). Extracted verbatim in Phase 0.2a
// (no behavior change); a later phase generates it from the precompile manifests
// plus the base op-type set.

/// The type can be: internal (no proof required), arith, binary, etc.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OpType {
    Internal,
    Arith,
    ArithA32,
    ArithAm32,
    Binary,
    BinaryE,
    Keccak,
    Sha256,
    Poseidon,
    PubOut,
    ArithEq,
    Fcall,
    ArithEq384,
    BigInt,
    Dma,
    Blake2,
    Profile,
}

impl From<OpType> for ZiskOperationType {
    fn from(op_type: OpType) -> Self {
        match op_type {
            OpType::Internal => ZiskOperationType::Internal,
            OpType::Arith | OpType::ArithA32 | OpType::ArithAm32 => ZiskOperationType::Arith,
            OpType::Binary => ZiskOperationType::Binary,
            OpType::BinaryE => ZiskOperationType::BinaryE,
            OpType::Keccak => ZiskOperationType::Keccak,
            OpType::Sha256 => ZiskOperationType::Sha256,
            OpType::Poseidon => ZiskOperationType::Poseidon,
            OpType::PubOut => ZiskOperationType::PubOut,
            OpType::ArithEq => ZiskOperationType::ArithEq,
            OpType::Fcall => ZiskOperationType::Fcall,
            OpType::ArithEq384 => ZiskOperationType::ArithEq384,
            OpType::BigInt => ZiskOperationType::BigInt,
            OpType::Dma => ZiskOperationType::Dma,
            OpType::Blake2 => ZiskOperationType::Blake2,
            OpType::Profile => ZiskOperationType::Profile,
        }
    }
}

impl Display for OpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal => write!(f, "i"),
            Self::Arith => write!(f, "a"),
            Self::ArithA32 => write!(f, "a32"),
            Self::ArithAm32 => write!(f, "am32"),
            Self::Binary => write!(f, "b"),
            Self::BinaryE => write!(f, "BinaryE"),
            Self::Keccak => write!(f, "Keccak"),
            Self::Sha256 => write!(f, "Sha256"),
            Self::Poseidon => write!(f, "Poseidon"),
            Self::PubOut => write!(f, "PubOut"),
            Self::ArithEq => write!(f, "Arith256"),
            Self::Fcall => write!(f, "Fcall"),
            Self::ArithEq384 => write!(f, "Arith384"),
            Self::BigInt => write!(f, "BigInt"),
            Self::Dma => write!(f, "Dma"),
            Self::Blake2 => write!(f, "Blake2"),
            Self::Profile => write!(f, "Profile"),
        }
    }
}

impl FromStr for OpType {
    type Err = InvalidOpTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "i" => Ok(Self::Internal),
            "a" => Ok(Self::Arith),
            "a32" => Ok(Self::ArithA32),
            "am32" => Ok(Self::ArithAm32),
            "b" => Ok(Self::Binary),
            "be" => Ok(Self::BinaryE),
            "k" => Ok(Self::Keccak),
            "s" => Ok(Self::Sha256),
            "p" => Ok(Self::Poseidon),
            "aeq" => Ok(Self::ArithEq),
            "fcall" => Ok(Self::Fcall),
            "aeq384" => Ok(Self::ArithEq384),
            "bint" => Ok(Self::BigInt),
            "dma" => Ok(Self::Dma),
            "bl" => Ok(Self::Blake2),
            "profile" => Ok(Self::Profile),
            _ => Err(InvalidOpTypeError),
        }
    }
}
