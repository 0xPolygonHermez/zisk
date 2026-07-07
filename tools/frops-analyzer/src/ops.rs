//! Operation metadata used by the analyzer.
//!
//! Everything here is derived from `zisk_core::zisk_ops::ZiskOp` so the analyzer never drifts from
//! the canonical opcode / cost / type definitions in `core/src/zisk_ops.rs`.

use zisk_core::zisk_ops::{OpType, ZiskOp};

/// The three FROPS tables, one per generated `*_frops.rs` source file.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FropsTable {
    Arith,
    BinaryBasic,
    BinaryExt,
}

impl FropsTable {
    pub fn all() -> [FropsTable; 3] {
        [FropsTable::Arith, FropsTable::BinaryBasic, FropsTable::BinaryExt]
    }
    pub fn key(self) -> &'static str {
        match self {
            FropsTable::Arith => "arith",
            FropsTable::BinaryBasic => "binary_basic",
            FropsTable::BinaryExt => "binary_extension",
        }
    }
}

/// State machine an operation is proved in. Used by the padding-aware area model: each instance of a
/// state machine has a fixed number of rows (the trace `NUM_ROWS`) and a fixed per-row area cost.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Sm {
    Arith,
    Binary,
    BinaryAdd,
    BinaryExt,
}

impl Sm {
    pub fn all() -> [Sm; 4] {
        [Sm::Arith, Sm::Binary, Sm::BinaryAdd, Sm::BinaryExt]
    }

    /// Rows per instance, mirroring the trace definitions in `pil/src/pil_helpers/traces.rs`.
    /// `ArithTrace` = 2^21, the binary traces = 2^22.
    pub fn num_rows(self) -> u64 {
        match self {
            Sm::Arith => 2_097_152,
            Sm::Binary | Sm::BinaryAdd | Sm::BinaryExt => 4_194_304,
        }
    }

    /// Per-row area cost. Uniform within a state machine (see `core/src/zisk_ops_costs.rs`).
    pub fn cost(self) -> u64 {
        match self {
            Sm::Arith => 95,     // ARITHA32_COST / ARITHAM32_COST
            Sm::Binary => 60,    // BINARY_COST
            Sm::BinaryAdd => 25, // BINARY_ADD_COST (only `add`, 0x0a)
            Sm::BinaryExt => 53, // BINARY_E_COST
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Sm::Arith => "Arith",
            Sm::Binary => "Binary",
            Sm::BinaryAdd => "BinaryAdd",
            Sm::BinaryExt => "BinaryExtension",
        }
    }
}

/// Metadata for a single opcode that is a FROPS candidate.
#[derive(Clone, Copy, Debug)]
pub struct OpInfo {
    pub code: u8,
    pub name: &'static str,
    pub cost: u64,
    pub table: FropsTable,
    pub sm: Sm,
}

/// Classifies an opcode into its FROPS table and state machine, or `None` if the opcode is not a
/// FROPS candidate (only Arith / Binary / BinaryE operations are).
pub fn classify(code: u8) -> Option<OpInfo> {
    let op = ZiskOp::try_from_code(code).ok()?;
    let (table, sm) = match op.op_type() {
        OpType::Arith | OpType::ArithA32 | OpType::ArithAm32 => (FropsTable::Arith, Sm::Arith),
        OpType::Binary => {
            let sm = if code == ZiskOp::Add.code() { Sm::BinaryAdd } else { Sm::Binary };
            (FropsTable::BinaryBasic, sm)
        }
        OpType::BinaryE => (FropsTable::BinaryExt, Sm::BinaryExt),
        _ => return None,
    };
    Some(OpInfo { code, name: op.name(), cost: op.cost(), table, sm })
}

/// `ZiskOp` enum-variant identifier (e.g. `Mulu`, `Add`) for a code, used to emit
/// `ZiskOp::<Variant>.code()` in generated source. Returns the debug name of the enum variant.
pub fn variant_ident(code: u8) -> Option<String> {
    let op = ZiskOp::try_from_code(code).ok()?;
    // `Debug` for the enum prints the variant identifier exactly (e.g. "Mulu", "SignExtendB").
    Some(format!("{op:?}"))
}
