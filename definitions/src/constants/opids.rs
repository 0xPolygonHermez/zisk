//! Bus / table / continuation ids used by the PIL constraints (and, for a few, by the
//! Rust state machines). Migrated from the hand-written `pil/opids.pil` + the Rust
//! mirror `pil/src/constants.rs`.
//!
//! Most ids are PIL-only (`to(pil)`). The five DMA / range ids the Rust DMA state
//! machines also use — with the same value — are `to(rust, pil)`. NOTE: `OPERATION_BUS_ID`
//! / `ROM_BUS_ID` are PIL-only here; Rust's `BusId(0)`/`BusId(1)` are the executor's
//! in-process routing tags, a different id space that happens to share the name.

use zisk_definitions_macros::constants;

#[constants(group = "opids", to(pil), dec)]
pub mod opids {
    /// Operation bus id (PIL constraint bus; distinct from the Rust `OPERATION_BUS_ID` BusId).
    pub const OPERATION_BUS_ID: usize = 5000;

    pub const MAIN_CONTINUATION_ID: usize = 1000;
    pub const MEMORY_CONTINUATION_ID: usize = 11;
    pub const ROM_DATA_CONTINUATION_ID: usize = 12;

    /// ROM bus id (PIL; distinct from the Rust `ROM_BUS_ID` BusId).
    pub const ROM_BUS_ID: usize = 7890;

    pub const MEMORY_ID: usize = 10;
    pub const MEMORY_ALIGN_ROM_ID: usize = 133;

    pub const ARITH_TABLE_ID: usize = 331;
    pub const ARITH_RANGE_TABLE_ID: usize = 330;

    pub const BINARY_TABLE_ID: usize = 125;
    pub const BINARY_EXTENSION_TABLE_ID: usize = 124;

    pub const ARITH_FROPS_TABLE_ID: usize = 5010;
    pub const BINARY_FROPS_TABLE_ID: usize = 5011;
    pub const BINARY_EXTENSION_FROPS_TABLE_ID: usize = 5012;

    pub const ARITH_EQ_LT_TABLE_ID: usize = 5002;
    pub const KECCAKF_TABLE_ID: usize = 126;
    pub const BLAKE2BR_PERMUTATION_ID: usize = 127;

    pub const DMA_BUS_ID: usize = 8000;

    // These five are also used by the Rust DMA state machines (same value) — emit to both.
    /// DMA rom id.
    #[emit(to(rust, pil))]
    pub const DMA_ROM_ID: usize = 8001;
    /// DMA pre/post table id.
    #[emit(to(rust, pil))]
    pub const DMA_PRE_POST_TABLE_ID: usize = 8002;
    /// DMA byte-compare table id.
    #[emit(to(rust, pil))]
    pub const DMA_BYTE_CMP_TABLE_ID: usize = 8003;
    /// Dual-range 7-bit id.
    #[emit(to(rust, pil))]
    pub const DUAL_RANGE_7_BITS_ID: usize = 77;
    /// Dual-range byte id.
    #[emit(to(rust, pil))]
    pub const DUAL_RANGE_BYTE_ID: usize = 88;
}

pub use opids::{EXPORTS, GROUP};
