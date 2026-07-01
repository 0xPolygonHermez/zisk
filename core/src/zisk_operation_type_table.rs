// `ZiskOperationType` (the op-type the emulator/bus route on) + its `*_OP_TYPE_ID`
// constants. `include!`d by `zisk_inst.rs` at item position. The enum is `#[repr(u32)]`
// and each `*_OP_TYPE_ID` is the variant's ordinal, so **declaration order is ABI**
// (it feeds bus routing + the proof system). Extracted verbatim in Phase 0.2a; a later
// phase generates it from the precompile manifests plus the base op-type set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
#[repr(u32)]
pub enum ZiskOperationType {
    None,
    Internal,
    // ZisK Core Operations
    Arith,
    Binary,
    BinaryE,
    Keccak,
    Sha256,
    Poseidon,
    Blake2,
    PubOut,
    ArithEq,
    ArithEq384,
    BigInt, // Note: Add new core operations here
    Dma,    // Note: To add extra params to precompiles calls
    // ZisK Free Input Operations
    FcallParam,
    Fcall,
    FcallGet,
    Profile,
}

pub const NONE_OP_TYPE_ID: u32 = ZiskOperationType::None as u32;
pub const INTERNAL_OP_TYPE_ID: u32 = ZiskOperationType::Internal as u32;
pub const ARITH_OP_TYPE_ID: u32 = ZiskOperationType::Arith as u32;
pub const BINARY_OP_TYPE_ID: u32 = ZiskOperationType::Binary as u32;
pub const BINARY_E_OP_TYPE_ID: u32 = ZiskOperationType::BinaryE as u32;
pub const KECCAK_OP_TYPE_ID: u32 = ZiskOperationType::Keccak as u32;
pub const SHA256_OP_TYPE_ID: u32 = ZiskOperationType::Sha256 as u32;
pub const POSEIDON_OP_TYPE_ID: u32 = ZiskOperationType::Poseidon as u32;
pub const PUB_OUT_OP_TYPE_ID: u32 = ZiskOperationType::PubOut as u32;
pub const ARITH_EQ_OP_TYPE_ID: u32 = ZiskOperationType::ArithEq as u32;
pub const ARITH_EQ_384_OP_TYPE_ID: u32 = ZiskOperationType::ArithEq384 as u32;
pub const BIG_INT_OP_TYPE_ID: u32 = ZiskOperationType::BigInt as u32;
pub const FCALL_PARAM_OP_TYPE_ID: u32 = ZiskOperationType::FcallParam as u32;
pub const FCALL_OP_TYPE_ID: u32 = ZiskOperationType::Fcall as u32;
pub const DMA_OP_TYPE_ID: u32 = ZiskOperationType::Dma as u32;
pub const BLAKE2_OP_TYPE_ID: u32 = ZiskOperationType::Blake2 as u32;
