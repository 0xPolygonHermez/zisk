//! The FROPS box data in `zisk_core::frops` and the generated predicates in this crate are two
//! renderings of the same analyzer output, and the ROM-histogram assembly counts frequent
//! operations from the boxes while the state machines prove them from the predicates. If the two
//! ever disagree, the assembly would count rows the state machines do not claim (or the other way
//! round), so this test pins them together.

use zisk_core::frops::{
    frops_regions, frops_row, FROPS_BINARY_BASIC_BASE, FROPS_BINARY_EXT_BASE, FROPS_TABLE_ROWS,
};
use zisk_sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};

mod common;
use common::samples;

/// Checks a family: every sampled triple must be a frequent operation for both renderings, and land
/// on the same table row. `base` translates a global row into this family's own table.
fn check(
    base: u64,
    is_frequent_op: fn(u8, u64, u64) -> bool,
    get_row: fn(u8, u64, u64) -> usize,
    no_frops: usize,
    ops: &[u8],
) {
    let mut checked = 0;
    for &op in ops {
        for (a, b) in samples(op) {
            let expected = frops_row(op, a, b);
            assert_eq!(
                is_frequent_op(op, a, b),
                expected.is_some(),
                "op {op:#04x} a={a:#x} b={b:#x}: predicate and boxes disagree"
            );
            match expected {
                Some(row) => assert_eq!(
                    get_row(op, a, b) as u64,
                    row - base,
                    "op {op:#04x} a={a:#x} b={b:#x}: different row"
                ),
                None => assert_eq!(get_row(op, a, b), no_frops, "op {op:#04x} a={a:#x} b={b:#x}"),
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "no FROPS opcodes to check");
}

/// Opcodes with boxes whose global rows fall inside `[base, base + len)`.
fn ops_of_family(base: u64, end: u64) -> Vec<u8> {
    (0u8..=255)
        .filter(|&op| {
            frops_regions(op).first().is_some_and(|r| r.base_row >= base && r.base_row < end)
        })
        .collect()
}

#[test]
fn binary_basic_predicates_match_the_boxes() {
    let ops = ops_of_family(FROPS_BINARY_BASIC_BASE, FROPS_BINARY_EXT_BASE);
    check(
        FROPS_BINARY_BASIC_BASE,
        BinaryBasicFrops::is_frequent_op,
        BinaryBasicFrops::get_row,
        BinaryBasicFrops::NO_FROPS,
        &ops,
    );
}

#[test]
fn binary_extension_predicates_match_the_boxes() {
    let ops = ops_of_family(FROPS_BINARY_EXT_BASE, u64::MAX);
    check(
        FROPS_BINARY_EXT_BASE,
        BinaryExtensionFrops::is_frequent_op,
        BinaryExtensionFrops::get_row,
        BinaryExtensionFrops::NO_FROPS,
        &ops,
    );
}

/// The family bases must match how many rows each family's table actually materialises. They are
/// what translates a global row of the assembly's column into a row of this family's own AIR, so a
/// drift here would publish every multiplicity at the wrong row without any other symptom.
#[test]
fn family_sizes_match_the_bases() {
    let mut basic = BinaryBasicFrops::new();
    basic.build_table();
    assert_eq!(
        basic.count() as u64,
        FROPS_BINARY_EXT_BASE - FROPS_BINARY_BASIC_BASE,
        "binary basic table size does not match the gap between the family bases"
    );

    let mut extension = BinaryExtensionFrops::new();
    extension.build_table();
    assert_eq!(
        extension.count() as u64,
        FROPS_TABLE_ROWS - FROPS_BINARY_EXT_BASE,
        "binary extension table size does not match the gap to the end of the column"
    );
}
