//! Shared by the FROPS agreement tests. A module under `tests/common/` is not a test target of
//! its own, so this file is compiled only as part of the tests that declare `mod common;`.
use zisk_core::frops::frops_regions;

/// `(a, b)` pairs that exercise every box of an opcode: the corners, a point inside, and the near
/// misses just outside each bound (including the stride alignment).
pub fn samples(op: u8) -> Vec<(u64, u64)> {
    let mut v = vec![(0, 0), (1, 1), (u64::MAX, u64::MAX), (u64::MAX, 0), (0, u64::MAX)];
    for r in frops_regions(op) {
        let a_last = r.a_lo + (r.a_count - 1) * r.a_stride;
        let b_last = r.b_lo + r.b_count - 1;
        let a_mid = r.a_lo + (r.a_count / 2) * r.a_stride;
        let b_mid = r.b_lo + r.b_count / 2;
        for a in [
            r.a_lo,
            a_mid,
            a_last,
            r.a_lo.wrapping_sub(1),
            a_last.wrapping_add(r.a_stride),
            r.a_lo.wrapping_add(1),
        ] {
            for b in [r.b_lo, b_mid, b_last, r.b_lo.wrapping_sub(1), b_last.wrapping_add(1)] {
                v.push((a, b));
            }
        }
    }
    v
}
