//! Lane addressing for the `Mem` PIL template.
//!
//! The `Mem` airtemplate packs `lanes_x_row` independent memory lanes on every
//! trace row (see `lanes_x_row` in `state-machines/mem/pil/mem.pil`). Each lane
//! carries the full set of columns that used to belong to a row — `addr`,
//! `step`, `sel`, `addr_changes`, `value`, `wr`, the increments and, with
//! `dual_mem`, its own `step_dual`/`sel_dual` — and the lane at index `l > 0`
//! chains from lane `l - 1` of the same row, while lane `0` chains from the last
//! lane of the previous row.
//!
//! Planning and the offsets table therefore work in **virtual rows**: one
//! virtual row per lane, numbered consecutively across the whole segment. A
//! virtual position `p` lives on the physical row `p / lanes_x_row`, at the lane
//! `p % lanes_x_row`.
//!
//! The number of lanes is never hardcoded here: [`mem_lanes_x_row`] and
//! [`input_data_lanes_x_row`] read it from the generated trace row (the length
//! of its column arrays), so the Rust side always follows the PIL.

use proofman_fields::Goldilocks;
use zisk_pil::{InputDataTraceRow, MemTraceRow, RomDataTraceRow};

/// Maps virtual positions to `(row, lane)` pairs.
///
/// `lanes_x_row` is required to be a power of two so the mapping is a shift and
/// a mask instead of a division in the witness inner loop.
#[derive(Clone, Copy, Debug)]
pub struct MemLanes {
    lanes: usize,
    shift: u32,
    mask: usize,
}

impl MemLanes {
    pub fn new(lanes: usize) -> Self {
        assert!(
            lanes.is_power_of_two(),
            "MemLanes: lanes_x_row must be a power of two, got {lanes}"
        );
        Self { lanes, shift: lanes.trailing_zeros(), mask: lanes - 1 }
    }

    /// Lanes packed on each physical row.
    #[inline(always)]
    pub fn lanes(&self) -> usize {
        self.lanes
    }

    /// Physical row and lane holding the virtual position `vpos`.
    #[inline(always)]
    pub fn split(&self, vpos: usize) -> (usize, usize) {
        (vpos >> self.shift, vpos & self.mask)
    }

    /// Number of virtual rows held by `num_rows` physical rows.
    #[inline(always)]
    pub fn slots(&self, num_rows: usize) -> usize {
        num_rows << self.shift
    }
}

/// `lanes_x_row` of the `Mem` air, read from the generated trace row.
pub fn mem_lanes_x_row() -> usize {
    MemTraceRow::<Goldilocks>::default().get_all_addr().len()
}

/// `lanes_x_row` of the `InputData` air, read from the generated trace row.
pub fn input_data_lanes_x_row() -> usize {
    InputDataTraceRow::<Goldilocks>::default().get_all_addr().len()
}

/// `lanes_x_row` of the `RomData` air, read from the generated trace row.
pub fn rom_data_lanes_x_row() -> usize {
    RomDataTraceRow::<Goldilocks>::default().get_all_addr().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_lane_is_the_identity() {
        let lanes = MemLanes::new(1);
        assert_eq!(lanes.lanes(), 1);
        assert_eq!(lanes.slots(2048), 2048);
        for vpos in 0..8 {
            assert_eq!(lanes.split(vpos), (vpos, 0));
        }
    }

    #[test]
    fn virtual_rows_walk_lanes_before_rows() {
        let lanes = MemLanes::new(4);
        assert_eq!(lanes.slots(1024), 4096);
        let walked: Vec<(usize, usize)> = (0..9).map(|v| lanes.split(v)).collect();
        assert_eq!(
            walked,
            vec![(0, 0), (0, 1), (0, 2), (0, 3), (1, 0), (1, 1), (1, 2), (1, 3), (2, 0),]
        );
    }

    #[test]
    fn split_matches_div_and_rem() {
        for &n in &[1usize, 2, 4, 8, 16] {
            let lanes = MemLanes::new(n);
            for vpos in 0..(4 * n + 3) {
                assert_eq!(lanes.split(vpos), (vpos / n, vpos % n), "lanes={n} vpos={vpos}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn rejects_non_power_of_two() {
        MemLanes::new(3);
    }
}
