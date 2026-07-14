//! DMA / range ids used by the Rust state machines.
//!
//! Values are single-sourced in `zisk-definitions` (`definitions/src/constants/opids.rs`,
//! generated to Rust + PIL by the constants sync). The generator emits them as `u64`
//! (ZisK is 64-bit); we re-type to `usize` here so the existing `zisk_pil::*` consumers
//! are unchanged. `DMA_PRE_POST_TABLE_SIZE` stays hand-written — its PIL source is
//! `dma_pre_post_table.pil` (`288 * 4`), not `opids`.

pub const DMA_ROM_ID: usize = zisk_definitions::opids::DMA_ROM_ID as usize;
pub const DMA_PRE_POST_TABLE_ID: usize = zisk_definitions::opids::DMA_PRE_POST_TABLE_ID as usize;
pub const DMA_BYTE_CMP_TABLE_ID: usize = zisk_definitions::opids::DMA_BYTE_CMP_TABLE_ID as usize;
pub const DUAL_RANGE_7_BITS_ID: usize = zisk_definitions::opids::DUAL_RANGE_7_BITS_ID as usize;
pub const DUAL_RANGE_BYTE_ID: usize = zisk_definitions::opids::DUAL_RANGE_BYTE_ID as usize;

pub const DMA_PRE_POST_TABLE_SIZE: usize = 1152;
