//! The kinds of binary operation the planner distributes, and the airs that prove each.
//!
//! The airs form two independent families — no air proves both a basic/add operation and an extension
//! one — so each family is distributed on its own.
//!
//! Every air comes in two heights: a plain one and a `Large` sibling twice as tall and exactly as
//! wide. They prove the same kinds, so what the strategy decides between them is only how many
//! instances the family opens — which is the first thing the criterion looks at (see
//! [`zisk_common::select_airs`]).

use crate::{lanes_x_row, AirSlot};
use zisk_pil::{
    BinaryAddHiHugeTrace, BinaryAddHiLargeTrace, BinaryAddHiTrace, BinaryAddHugeTrace,
    BinaryAddLargeTrace, BinaryAddTrace, BinaryExtensionLargeTrace, BinaryExtensionTrace,
    BinaryHugeTrace, BinaryLargeTrace, BinaryTrace, CompactBinaryTrace,
};

/// Kinds of the basic/add family, in the order the distributor sees them.
pub const ADD_KINDS: usize = 5;
/// Basic binary operations: only the `Binary` airs prove them. SH3ADD operations that neither
/// packed air can take (see [`crate::sh3add_shape`]) are basic operations to this family and are
/// counted here.
pub const KIND_BASIC: usize = 0;
/// Additions whose result fits in the low limb: the packed airs prove them too.
pub const KIND_ADD_HI: usize = 1;
/// Additions needing the full 64-bit add.
pub const KIND_ADD_FULL: usize = 2;
/// SH3ADD whose whole result fits in the low limb, so `BinaryAddHi` proves it too.
pub const KIND_SH3ADD_HI: usize = 3;
/// SH3ADD that only the full 64-bit add can take: a clean 32-bit shifted operand whose low limb
/// carries at most once.
pub const KIND_SH3ADD_ADD: usize = 4;

/// Kinds of the extension family. Both extension airs are instantiated `full`, so every extension
/// operation is one and the same kind to them.
pub const EXT_KINDS: usize = 1;
/// Every extension operation.
pub const KIND_EXT: usize = 0;

/// Airs of the add family, in hand-out order.
pub const ADD_AIRS: usize = 9;
/// Airs of the extension family, in hand-out order.
pub const EXT_AIRS: usize = 2;

/// Blocks of `CompactBinary` that take part in the add family's hand-out, ahead of its airs: the
/// `add_hi_`, `add_` and `basic_` blocks, in that order.
pub const COMPACT_ADD_BLOCKS: usize = 3;
/// Slot of each block within [`compact_add_blocks`].
pub const COMPACT_BLOCK_ADD_HI: usize = 0;
pub const COMPACT_BLOCK_ADD: usize = 1;
pub const COMPACT_BLOCK_BASIC: usize = 2;

/// Operations each block of one `CompactBinary` instance holds, i.e. its rows times the lanes the
/// block packs.
pub mod compact_capacity {
    use super::*;
    pub fn basic() -> u64 {
        (CompactBinaryTrace::<()>::NUM_ROWS * lanes_x_row::COMPACT_BASIC) as u64
    }
    pub fn add() -> u64 {
        (CompactBinaryTrace::<()>::NUM_ROWS * lanes_x_row::COMPACT_ADD) as u64
    }
    pub fn add_hi() -> u64 {
        (CompactBinaryTrace::<()>::NUM_ROWS * lanes_x_row::COMPACT_ADD_HI) as u64
    }
    pub fn ext() -> u64 {
        (CompactBinaryTrace::<()>::NUM_ROWS * lanes_x_row::COMPACT_EXT) as u64
    }
}

/// The three add-family blocks of `CompactBinary`, as the slots that go FIRST in the hand-out, so
/// the fused instance takes what fits in each block and the rest flows on to the standalone airs.
/// Every block names the fused air: the plans the hand-out gives them are merged into one.
///
/// Each block takes only the kinds its own air is the specialist of -- the `add_` block does not
/// take the low-limb additions the `add_hi_` block could not hold, nor the `basic_` block any
/// addition -- so what the fused instance ends up holding is exactly the share the strategy sized
/// it with: the minimum of each family and its block. `instances` is 1 when the strategy uses the
/// fused air and 0 otherwise, for the three blocks alike.
pub fn compact_add_blocks(instances: u64) -> [AirSlot<ADD_KINDS>; COMPACT_ADD_BLOCKS] {
    let block = |ops_per_instance, proves, sees| AirSlot {
        airgroup_id: CompactBinaryTrace::<()>::AIRGROUP_ID,
        air_id: CompactBinaryTrace::<()>::AIR_ID,
        ops_per_instance,
        proves,
        sees,
        instances,
    };
    // Kind order: [BASIC, ADD_HI, ADD_FULL, SH3ADD_HI, SH3ADD_ADD]. What each block sees is what
    // the collector of its air sees, since the fused instance collects through those collectors.
    [
        block(
            compact_capacity::add_hi(),
            [false, true, false, true, false],
            [false, true, true, true, true],
        ),
        block(
            compact_capacity::add(),
            [false, false, true, false, true],
            [false, true, true, true, true],
        ),
        block(compact_capacity::basic(), [true, false, false, false, false], [true; ADD_KINDS]),
    ]
}

/// The `ext_` block of `CompactBinary`, the slot that goes first in the extension family's hand-out.
pub fn compact_ext_block(instances: u64) -> AirSlot<EXT_KINDS> {
    AirSlot {
        airgroup_id: CompactBinaryTrace::<()>::AIRGROUP_ID,
        air_id: CompactBinaryTrace::<()>::AIR_ID,
        ops_per_instance: compact_capacity::ext(),
        proves: [true],
        sees: [true],
        instances,
    }
}

/// The add-family airs, most specific and tallest first, so each takes what it can and the rest flows
/// on. Within a specialisation the tall air goes first because filling it is what spares the family an
/// instance.
///
/// `instances` are the counts the strategy granted, in the same order.
pub fn add_family(instances: [u64; ADD_AIRS]) -> [AirSlot<ADD_KINDS>; ADD_AIRS] {
    // Kind order: [BASIC, ADD_HI, ADD_FULL, SH3ADD_HI, SH3ADD_ADD].
    let packed = |airgroup_id, air_id, ops_per_instance, instances| AirSlot {
        airgroup_id,
        air_id,
        ops_per_instance,
        proves: [false, true, false, true, false],
        // Its collector filters by opcode, not by shape, so it also sees the shapes it cannot prove.
        sees: [false, true, true, true, true],
        instances,
    };
    let add = |airgroup_id, air_id, ops_per_instance, instances| AirSlot {
        airgroup_id,
        air_id,
        ops_per_instance,
        proves: [false, true, true, true, true],
        sees: [false, true, true, true, true],
        instances,
    };
    let basic = |airgroup_id, air_id, ops_per_instance, instances| AirSlot {
        airgroup_id,
        air_id,
        ops_per_instance,
        proves: [true, true, true, true, true],
        sees: [true, true, true, true, true],
        instances,
    };

    // Operations one instance holds: rows times the lanes the air packs on each of them.
    let ops = |rows: usize, lanes: usize| (rows * lanes) as u64;

    [
        packed(
            BinaryAddHiHugeTrace::<()>::AIRGROUP_ID,
            BinaryAddHiHugeTrace::<()>::AIR_ID,
            ops(BinaryAddHiHugeTrace::<()>::NUM_ROWS, lanes_x_row::ADD_HI_HUGE),
            instances[0],
        ),
        packed(
            BinaryAddHiLargeTrace::<()>::AIRGROUP_ID,
            BinaryAddHiLargeTrace::<()>::AIR_ID,
            ops(BinaryAddHiLargeTrace::<()>::NUM_ROWS, lanes_x_row::ADD_HI_LARGE),
            instances[1],
        ),
        packed(
            BinaryAddHiTrace::<()>::AIRGROUP_ID,
            BinaryAddHiTrace::<()>::AIR_ID,
            ops(BinaryAddHiTrace::<()>::NUM_ROWS, lanes_x_row::ADD_HI),
            instances[2],
        ),
        add(
            BinaryAddHugeTrace::<()>::AIRGROUP_ID,
            BinaryAddHugeTrace::<()>::AIR_ID,
            ops(BinaryAddHugeTrace::<()>::NUM_ROWS, lanes_x_row::ADD_HUGE),
            instances[3],
        ),
        add(
            BinaryAddLargeTrace::<()>::AIRGROUP_ID,
            BinaryAddLargeTrace::<()>::AIR_ID,
            ops(BinaryAddLargeTrace::<()>::NUM_ROWS, lanes_x_row::ADD_LARGE),
            instances[4],
        ),
        add(
            BinaryAddTrace::<()>::AIRGROUP_ID,
            BinaryAddTrace::<()>::AIR_ID,
            ops(BinaryAddTrace::<()>::NUM_ROWS, lanes_x_row::ADD),
            instances[5],
        ),
        basic(
            BinaryHugeTrace::<()>::AIRGROUP_ID,
            BinaryHugeTrace::<()>::AIR_ID,
            ops(BinaryHugeTrace::<()>::NUM_ROWS, lanes_x_row::BASIC_HUGE),
            instances[6],
        ),
        basic(
            BinaryLargeTrace::<()>::AIRGROUP_ID,
            BinaryLargeTrace::<()>::AIR_ID,
            ops(BinaryLargeTrace::<()>::NUM_ROWS, lanes_x_row::BASIC_LARGE),
            instances[7],
        ),
        basic(
            BinaryTrace::<()>::AIRGROUP_ID,
            BinaryTrace::<()>::AIR_ID,
            ops(BinaryTrace::<()>::NUM_ROWS, lanes_x_row::BASIC),
            instances[8],
        ),
    ]
}

/// The extension-family airs, the widest first so it fills before a narrower one is opened. Both
/// prove every extension operation, so the only thing that tells them apart is how many they
/// pack per row.
pub fn ext_family(instances: [u64; EXT_AIRS]) -> [AirSlot<EXT_KINDS>; EXT_AIRS] {
    let ext = |airgroup_id, air_id, rows: usize, lanes: usize, instances| AirSlot {
        airgroup_id,
        air_id,
        ops_per_instance: (rows * lanes) as u64,
        proves: [true],
        sees: [true],
        instances,
    };

    [
        ext(
            BinaryExtensionLargeTrace::<()>::AIRGROUP_ID,
            BinaryExtensionLargeTrace::<()>::AIR_ID,
            BinaryExtensionLargeTrace::<()>::NUM_ROWS,
            lanes_x_row::EXT_LARGE,
            instances[0],
        ),
        ext(
            BinaryExtensionTrace::<()>::AIRGROUP_ID,
            BinaryExtensionTrace::<()>::AIR_ID,
            BinaryExtensionTrace::<()>::NUM_ROWS,
            lanes_x_row::EXT,
            instances[1],
        ),
    ]
}
