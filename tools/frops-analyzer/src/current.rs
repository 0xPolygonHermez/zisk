//! Bridge to the *current* (hand-written / previously generated) FROPS implementation, so the
//! analyzer can score its proposal against what is already in the tree.

use std::collections::HashMap;

use zisk_sm_arith::ArithFrops;
use zisk_sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};

use crate::ops::FropsTable;

/// Does the current implementation treat `(op, a, b)` of the given table as a frequent op?
#[inline]
pub fn is_frequent(table: FropsTable, op: u8, a: u64, b: u64) -> bool {
    match table {
        FropsTable::Arith => ArithFrops::is_frequent_op(op, a, b),
        FropsTable::BinaryBasic => BinaryBasicFrops::is_frequent_op(op, a, b),
        FropsTable::BinaryExt => BinaryExtensionFrops::is_frequent_op(op, a, b),
    }
}

/// Row count of each current table (built once from the current `build_table`).
pub fn table_rows() -> HashMap<FropsTable, u64> {
    let mut arith = ArithFrops::new();
    arith.build_table();
    let mut basic = BinaryBasicFrops::new();
    basic.build_table();
    let mut ext = BinaryExtensionFrops::new();
    ext.build_table();

    let mut m = HashMap::new();
    m.insert(FropsTable::Arith, arith.count() as u64);
    m.insert(FropsTable::BinaryBasic, basic.count() as u64);
    m.insert(FropsTable::BinaryExt, ext.count() as u64);
    m
}
