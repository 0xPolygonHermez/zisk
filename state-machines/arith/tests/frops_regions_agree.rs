//! The FROPS box data in `zisk_core::frops` and the generated predicates in this crate are two
//! renderings of the same analyzer output, and the ROM-histogram assembly counts frequent
//! operations from the boxes while the state machines prove them from the predicates. If the two
//! ever disagree, the assembly would count rows the state machines do not claim (or the other way
//! round), so this test pins them together.

use zisk_core::frops::{frops_regions, frops_row, FROPS_ARITH_BASE, FROPS_BINARY_BASIC_BASE};
use zisk_sm_arith::ArithFrops;

mod common;
use common::samples;

#[test]
fn arith_predicates_match_the_boxes() {
    let ops: Vec<u8> = (0u8..=255)
        .filter(|&op| {
            // Arith is the first family of the global column, so its base is row 0 and only the
            // upper bound has to be tested.
            frops_regions(op).first().is_some_and(|r| r.base_row < FROPS_BINARY_BASIC_BASE)
        })
        .collect();
    let mut checked = 0;
    for &op in &ops {
        for (a, b) in samples(op) {
            let expected = frops_row(op, a, b);
            assert_eq!(
                ArithFrops::is_frequent_op(op, a, b),
                expected.is_some(),
                "op {op:#04x} a={a:#x} b={b:#x}: predicate and boxes disagree"
            );
            match expected {
                Some(row) => assert_eq!(
                    ArithFrops::get_row(op, a, b) as u64,
                    row - FROPS_ARITH_BASE,
                    "op {op:#04x} a={a:#x} b={b:#x}: different row"
                ),
                None => assert_eq!(
                    ArithFrops::get_row(op, a, b),
                    ArithFrops::NO_FROPS,
                    "op {op:#04x} a={a:#x} b={b:#x}"
                ),
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "no FROPS opcodes to check");
}

/// See the note in the binary crate's equivalent test: the family bases translate a global row of
/// the assembly's column into this family's own AIR row.
#[test]
fn family_size_matches_the_base() {
    let mut arith = ArithFrops::new();
    arith.build_table();
    assert_eq!(
        arith.count() as u64,
        FROPS_BINARY_BASIC_BASE - FROPS_ARITH_BASE,
        "arith table size does not match the gap between the family bases"
    );
}
