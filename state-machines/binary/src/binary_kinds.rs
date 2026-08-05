//! The kinds of binary operation the planner distributes, and the airs that prove each.
//!
//! The airs form two independent families — no air proves both a basic/add operation and an extension
//! one — so each family is distributed on its own.

use crate::AirSlot;
use zisk_pil::{
    BinaryAddHiTrace, BinaryAddTrace, BinaryExtensionFullTrace, BinaryExtensionTrace, BinaryTrace,
};

/// Kinds of the basic/add family, in the order the distributor sees them.
pub const ADD_KINDS: usize = 3;
/// Basic binary operations: only `Binary` proves them.
pub const KIND_BASIC: usize = 0;
/// Additions whose result fits in the low limb: the packed air proves them too.
pub const KIND_ADD_HI: usize = 1;
/// Additions needing the full 64-bit add.
pub const KIND_ADD_FULL: usize = 2;

/// Kinds of the extension family.
pub const EXT_KINDS: usize = 2;
/// Extension operations the reduced air can prove.
pub const KIND_EXT_CLEAN: usize = 0;
/// Extension operations that need the full air.
pub const KIND_EXT_DIRTY: usize = 1;

/// The add-family airs, most specific first, so each takes what it can and the rest flows on.
pub fn add_family(add_hi: u64, add: u64, basic: u64) -> [AirSlot<ADD_KINDS>; 3] {
    [
        AirSlot {
            airgroup_id: BinaryAddHiTrace::<()>::AIRGROUP_ID,
            air_id: BinaryAddHiTrace::<()>::AIR_ID,
            ops_per_instance: crate::ADDS_X_ROW as u64 * BinaryAddHiTrace::<()>::NUM_ROWS as u64,
            proves: [false, true, false],
            instances: add_hi,
        },
        AirSlot {
            airgroup_id: BinaryAddTrace::<()>::AIRGROUP_ID,
            air_id: BinaryAddTrace::<()>::AIR_ID,
            ops_per_instance: BinaryAddTrace::<()>::NUM_ROWS as u64,
            proves: [false, true, true],
            instances: add,
        },
        AirSlot {
            airgroup_id: BinaryTrace::<()>::AIRGROUP_ID,
            air_id: BinaryTrace::<()>::AIR_ID,
            ops_per_instance: BinaryTrace::<()>::NUM_ROWS as u64,
            proves: [true, true, true],
            instances: basic,
        },
    ]
}

/// The extension-family airs, the reduced one first so the clean operations fill it and the full one
/// takes whatever is left along with the dirty ones.
pub fn ext_family(reduced: u64, full: u64) -> [AirSlot<EXT_KINDS>; 2] {
    [
        AirSlot {
            airgroup_id: BinaryExtensionTrace::<()>::AIRGROUP_ID,
            air_id: BinaryExtensionTrace::<()>::AIR_ID,
            ops_per_instance: BinaryExtensionTrace::<()>::NUM_ROWS as u64,
            proves: [true, false],
            instances: reduced,
        },
        AirSlot {
            airgroup_id: BinaryExtensionFullTrace::<()>::AIRGROUP_ID,
            air_id: BinaryExtensionFullTrace::<()>::AIR_ID,
            ops_per_instance: BinaryExtensionFullTrace::<()>::NUM_ROWS as u64,
            proves: [true, true],
            instances: full,
        },
    ]
}
