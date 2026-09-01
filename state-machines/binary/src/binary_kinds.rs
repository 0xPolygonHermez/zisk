//! The kinds of binary operation the planner distributes, and the airs that prove each.
//!
//! The airs form two independent families — no air proves both a basic/add operation and an extension
//! one — so each family is distributed on its own.
//!
//! Every air comes in two heights: a plain one and a `Large` sibling twice as tall and exactly as
//! wide. They prove the same kinds, so what the strategy decides between them is only how many
//! instances the family opens — which is the first thing the criterion looks at (see
//! [`zisk_common::select_airs`]).

use crate::{AirSlot, ADDS_X_ROW, ADDS_X_ROW_LARGE};
use zisk_pil::{
    BinaryAddHiLargeTrace, BinaryAddHiTrace, BinaryAddLargeTrace, BinaryAddTrace,
    BinaryExtensionLargeTrace, BinaryExtensionTrace, BinaryLargeTrace, BinaryTrace,
};

/// Kinds of the basic/add family, in the order the distributor sees them.
pub const ADD_KINDS: usize = 3;
/// Basic binary operations: only the `Binary` airs prove them.
pub const KIND_BASIC: usize = 0;
/// Additions whose result fits in the low limb: the packed airs prove them too.
pub const KIND_ADD_HI: usize = 1;
/// Additions needing the full 64-bit add.
pub const KIND_ADD_FULL: usize = 2;

/// Kinds of the extension family. Both extension airs are instantiated `full`, so every extension
/// operation is one and the same kind to them.
pub const EXT_KINDS: usize = 1;
/// Every extension operation.
pub const KIND_EXT: usize = 0;

/// Airs of the add family, in hand-out order.
pub const ADD_AIRS: usize = 6;
/// Airs of the extension family, in hand-out order.
pub const EXT_AIRS: usize = 2;

/// The add-family airs, most specific and tallest first, so each takes what it can and the rest flows
/// on. Within a specialisation the tall air goes first because filling it is what spares the family an
/// instance.
///
/// `instances` are the counts the strategy granted, in the same order.
pub fn add_family(instances: [u64; ADD_AIRS]) -> [AirSlot<ADD_KINDS>; ADD_AIRS] {
    let packed = |airgroup_id, air_id, rows: usize, adds_x_row: usize, instances| AirSlot {
        airgroup_id,
        air_id,
        ops_per_instance: adds_x_row as u64 * rows as u64,
        proves: [false, true, false],
        // Its collector filters by `op == Add`, so it also sees the full-shape additions.
        sees: [false, true, true],
        instances,
    };
    let add = |airgroup_id, air_id, rows: usize, instances| AirSlot {
        airgroup_id,
        air_id,
        ops_per_instance: rows as u64,
        proves: [false, true, true],
        sees: [false, true, true],
        instances,
    };
    let basic = |airgroup_id, air_id, rows: usize, instances| AirSlot {
        airgroup_id,
        air_id,
        ops_per_instance: rows as u64,
        proves: [true, true, true],
        sees: [true, true, true],
        instances,
    };

    [
        packed(
            BinaryAddHiLargeTrace::<()>::AIRGROUP_ID,
            BinaryAddHiLargeTrace::<()>::AIR_ID,
            BinaryAddHiLargeTrace::<()>::NUM_ROWS,
            ADDS_X_ROW_LARGE,
            instances[0],
        ),
        packed(
            BinaryAddHiTrace::<()>::AIRGROUP_ID,
            BinaryAddHiTrace::<()>::AIR_ID,
            BinaryAddHiTrace::<()>::NUM_ROWS,
            ADDS_X_ROW,
            instances[1],
        ),
        add(
            BinaryAddLargeTrace::<()>::AIRGROUP_ID,
            BinaryAddLargeTrace::<()>::AIR_ID,
            BinaryAddLargeTrace::<()>::NUM_ROWS,
            instances[2],
        ),
        add(
            BinaryAddTrace::<()>::AIRGROUP_ID,
            BinaryAddTrace::<()>::AIR_ID,
            BinaryAddTrace::<()>::NUM_ROWS,
            instances[3],
        ),
        basic(
            BinaryLargeTrace::<()>::AIRGROUP_ID,
            BinaryLargeTrace::<()>::AIR_ID,
            BinaryLargeTrace::<()>::NUM_ROWS,
            instances[4],
        ),
        basic(
            BinaryTrace::<()>::AIRGROUP_ID,
            BinaryTrace::<()>::AIR_ID,
            BinaryTrace::<()>::NUM_ROWS,
            instances[5],
        ),
    ]
}

/// The extension-family airs, the tall one first so it fills before the short one is opened. Both
/// prove every extension operation, so the only thing that tells them apart is their height.
pub fn ext_family(instances: [u64; EXT_AIRS]) -> [AirSlot<EXT_KINDS>; EXT_AIRS] {
    [
        AirSlot {
            airgroup_id: BinaryExtensionLargeTrace::<()>::AIRGROUP_ID,
            air_id: BinaryExtensionLargeTrace::<()>::AIR_ID,
            ops_per_instance: BinaryExtensionLargeTrace::<()>::NUM_ROWS as u64,
            proves: [true],
            sees: [true],
            instances: instances[0],
        },
        AirSlot {
            airgroup_id: BinaryExtensionTrace::<()>::AIRGROUP_ID,
            air_id: BinaryExtensionTrace::<()>::AIR_ID,
            ops_per_instance: BinaryExtensionTrace::<()>::NUM_ROWS as u64,
            proves: [true],
            sees: [true],
            instances: instances[1],
        },
    ]
}
