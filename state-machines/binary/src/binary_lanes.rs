//! Lane addressing for the binary airs.
//!
//! Every binary air packs `lanes_x_row` independent operations on each row (see `lanes_x_row` in
//! `state-machines/binary/pil/*.pil`). A lane carries the full set of columns that proves ONE
//! operation, and nothing in these airs relates a row to its neighbour, so the packing is a pure
//! widening: a lane is a slot, and slots are filled in order.
//!
//! The witness therefore works in **slots**: one slot per lane, numbered consecutively across the
//! whole instance. Slot `s` lives on row `s / lanes_x_row`, at lane `s % lanes_x_row`.
//!
//! Unlike `MemLanes`, the lane count here is NOT required to be a power of two: nothing maps a slot
//! with a shift and a mask, so the split is a plain division. It is done once per instance and
//! hoisted out of the fill loop, never per operation.
//!
//! The number of lanes is never hardcoded. Callers build this from the generated trace row itself —
//! `BinaryLanes::new(R::default().get_all_a().len())` — because the length of a per-lane column
//! array IS `lanes_x_row`, so the Rust side always follows the PIL with no constant to keep in step.

/// Operations each air packs on a row, i.e. its `lanes_x_row` in `pil/zisk.pil`.
///
/// These are consts because the planner needs them before any trace exists, but they are not taken
/// on trust: [`tests::the_constants_match_the_generated_rows`] checks every one against the length
/// of its air's per-lane column arrays, which IS `lanes_x_row`. A change in the PIL that is not
/// mirrored here fails that test rather than corrupting a trace.
pub mod lanes_x_row {
    /// `Binary`, `BinaryLarge`, `BinaryHuge`.
    pub const BASIC: usize = 1;
    pub const BASIC_LARGE: usize = 2;
    pub const BASIC_HUGE: usize = 4;

    /// `BinaryAdd`, `BinaryAddLarge`, `BinaryAddHuge`.
    pub const ADD: usize = 1;
    pub const ADD_LARGE: usize = 2;
    pub const ADD_HUGE: usize = 4;

    /// `BinaryAddHi`, `BinaryAddHiLarge`, `BinaryAddHiHuge`.
    pub const ADD_HI: usize = 2;
    pub const ADD_HI_LARGE: usize = 4;
    pub const ADD_HI_HUGE: usize = 8;

    /// `BinaryExtension`, `BinaryExtensionLarge`.
    pub const EXT: usize = 1;
    pub const EXT_LARGE: usize = 2;

    /// The widest packing any add-hi air uses, so one row can be built through a fixed-size buffer
    /// whatever air is being filled.
    pub const MAX_ADD_HI: usize = ADD_HI_HUGE;

    /// The blocks of `CompactBinary`: lanes of the four airs side by side on one row, i.e. the
    /// `*_lanes_x_row` arguments of its instantiation in `pil/zisk.pil`.
    pub const COMPACT_BASIC: usize = 1;
    pub const COMPACT_ADD: usize = 1;
    pub const COMPACT_ADD_HI: usize = 2;
    pub const COMPACT_EXT: usize = 1;
}

/// Maps slots to `(row, lane)` pairs for one air.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryLanes {
    lanes: usize,
}

impl BinaryLanes {
    /// Builds the mapping for an air packing `lanes` operations per row.
    ///
    /// # Panics
    /// Panics if `lanes` is zero: an air that holds no operation per row could never be filled, and
    /// every division below would trap.
    pub fn new(lanes: usize) -> Self {
        assert!(lanes > 0, "BinaryLanes: lanes_x_row must be greater than 0");
        Self { lanes }
    }

    /// Operations packed on each row.
    #[inline(always)]
    pub fn lanes(&self) -> usize {
        self.lanes
    }

    /// Row and lane holding slot `slot`.
    #[inline(always)]
    pub fn split(&self, slot: usize) -> (usize, usize) {
        (slot / self.lanes, slot % self.lanes)
    }

    /// Slots held by `num_rows` rows, i.e. the operations one instance can prove.
    #[inline(always)]
    pub fn slots(&self, num_rows: usize) -> usize {
        num_rows * self.lanes
    }

    /// Rows needed to hold `slots` operations, padding the last one.
    #[inline(always)]
    pub fn rows_for(&self, slots: usize) -> usize {
        slots.div_ceil(self.lanes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The consts drive the planner, the generated rows drive the witness, and a disagreement
    /// between them would show up as a trace that does not match its air. Checking them here is
    /// what keeps `pil/zisk.pil` the single source of truth.
    #[test]
    fn the_constants_match_the_generated_rows() {
        use proofman_fields::Goldilocks;
        use zisk_pil::*;

        macro_rules! check {
            ($row:ident, $probe:ident, $konst:path) => {
                assert_eq!(
                    $row::<Goldilocks>::default().$probe().len(),
                    $konst,
                    concat!(stringify!($konst), " does not match ", stringify!($row)),
                );
            };
        }

        check!(BinaryTraceRow, get_all_b_op, lanes_x_row::BASIC);
        check!(BinaryLargeTraceRow, get_all_b_op, lanes_x_row::BASIC_LARGE);
        check!(BinaryHugeTraceRow, get_all_b_op, lanes_x_row::BASIC_HUGE);

        check!(CompactBinaryTraceRow, get_all_basic_b_op, lanes_x_row::COMPACT_BASIC);
        check!(CompactBinaryTraceRow, get_all_add_a, lanes_x_row::COMPACT_ADD);
        check!(CompactBinaryTraceRow, get_all_add_hi_a, lanes_x_row::COMPACT_ADD_HI);
        check!(CompactBinaryTraceRow, get_all_ext_op, lanes_x_row::COMPACT_EXT);

        check!(BinaryAddTraceRow, get_all_a, lanes_x_row::ADD);
        check!(BinaryAddLargeTraceRow, get_all_a, lanes_x_row::ADD_LARGE);
        check!(BinaryAddHugeTraceRow, get_all_a, lanes_x_row::ADD_HUGE);

        check!(BinaryAddHiTraceRow, get_all_a, lanes_x_row::ADD_HI);
        check!(BinaryAddHiLargeTraceRow, get_all_a, lanes_x_row::ADD_HI_LARGE);
        check!(BinaryAddHiHugeTraceRow, get_all_a, lanes_x_row::ADD_HI_HUGE);

        check!(BinaryExtensionTraceRow, get_all_op, lanes_x_row::EXT);
        check!(BinaryExtensionLargeTraceRow, get_all_op, lanes_x_row::EXT_LARGE);

        // The extension airs are instantiated with every lane `full`, and the witness relies on it:
        // it writes the full-only columns of every slot without asking whether the lane owns them.
        // Those columns are sized `full`, so a `full` below `lanes_x_row` in `zisk.pil` would make
        // them shorter than the lane count — which is exactly what this catches.
        check!(BinaryExtensionTraceRow, get_all_op_is_chain, lanes_x_row::EXT);
        check!(BinaryExtensionLargeTraceRow, get_all_op_is_chain, lanes_x_row::EXT_LARGE);
        check!(BinaryExtensionTraceRow, get_all_b, lanes_x_row::EXT);
        check!(BinaryExtensionLargeTraceRow, get_all_b, lanes_x_row::EXT_LARGE);
    }

    /// The row traits the state machines are generic over must report the same packing width as the
    /// air they fill. `the_constants_match_the_generated_rows` pins the constants against the rows;
    /// this pins the STATE MACHINES against the constants, which is the other half — a hardcoded
    /// width in a macro invocation would otherwise index past the end of a row and only show up as
    /// a panic deep inside witness computation.
    #[test]
    fn the_state_machines_pack_what_their_air_holds() {
        use crate::{
            BinaryAddHiLaneRow, BinaryAddLaneRow, BinaryBasicLaneRow, BinaryExtensionLaneRow,
        };
        use proofman_fields::Goldilocks;
        use zisk_pil::*;

        type F = Goldilocks;

        macro_rules! check {
            ($trait:ident, $row:ident, $trace:ident, $probe:ident, $konst:path) => {
                assert_eq!(
                    <$row<F> as $trait<F>>::LANES_X_ROW,
                    $row::<F>::default().$probe().len(),
                    concat!(stringify!($row), " packs a different width than its air"),
                );
                assert_eq!(
                    <$row<F> as $trait<F>>::LANES_X_ROW,
                    $konst,
                    concat!(stringify!($row), " does not use ", stringify!($konst)),
                );
            };
        }

        check!(BinaryBasicLaneRow, BinaryTraceRow, BinaryTrace, get_all_b_op, lanes_x_row::BASIC);
        check!(
            BinaryBasicLaneRow,
            BinaryLargeTraceRow,
            BinaryLargeTrace,
            get_all_b_op,
            lanes_x_row::BASIC_LARGE
        );
        check!(
            BinaryBasicLaneRow,
            BinaryHugeTraceRow,
            BinaryHugeTrace,
            get_all_b_op,
            lanes_x_row::BASIC_HUGE
        );

        check!(BinaryAddLaneRow, BinaryAddTraceRow, BinaryAddTrace, get_all_a, lanes_x_row::ADD);
        check!(
            BinaryAddLaneRow,
            BinaryAddLargeTraceRow,
            BinaryAddLargeTrace,
            get_all_a,
            lanes_x_row::ADD_LARGE
        );
        check!(
            BinaryAddLaneRow,
            BinaryAddHugeTraceRow,
            BinaryAddHugeTrace,
            get_all_a,
            lanes_x_row::ADD_HUGE
        );

        check!(
            BinaryAddHiLaneRow,
            BinaryAddHiTraceRow,
            BinaryAddHiTrace,
            get_all_a,
            lanes_x_row::ADD_HI
        );
        check!(
            BinaryAddHiLaneRow,
            BinaryAddHiLargeTraceRow,
            BinaryAddHiLargeTrace,
            get_all_a,
            lanes_x_row::ADD_HI_LARGE
        );
        check!(
            BinaryAddHiLaneRow,
            BinaryAddHiHugeTraceRow,
            BinaryAddHiHugeTrace,
            get_all_a,
            lanes_x_row::ADD_HI_HUGE
        );

        check!(
            BinaryExtensionLaneRow,
            BinaryExtensionTraceRow,
            BinaryExtensionTrace,
            get_all_op,
            lanes_x_row::EXT
        );
        check!(
            BinaryExtensionLaneRow,
            BinaryExtensionLargeTraceRow,
            BinaryExtensionLargeTrace,
            get_all_op,
            lanes_x_row::EXT_LARGE
        );

        // The fused row, block by block.
        check!(
            BinaryBasicLaneRow,
            CompactBinaryTraceRow,
            CompactBinaryTrace,
            get_all_basic_b_op,
            lanes_x_row::COMPACT_BASIC
        );
        check!(
            BinaryAddLaneRow,
            CompactBinaryTraceRow,
            CompactBinaryTrace,
            get_all_add_a,
            lanes_x_row::COMPACT_ADD
        );
        check!(
            BinaryAddHiLaneRow,
            CompactBinaryTraceRow,
            CompactBinaryTrace,
            get_all_add_hi_a,
            lanes_x_row::COMPACT_ADD_HI
        );
        check!(
            BinaryExtensionLaneRow,
            CompactBinaryTraceRow,
            CompactBinaryTrace,
            get_all_ext_op,
            lanes_x_row::COMPACT_EXT
        );
    }

    #[test]
    fn single_lane_is_the_identity() {
        let lanes = BinaryLanes::new(1);
        assert_eq!(lanes.lanes(), 1);
        assert_eq!(lanes.slots(2048), 2048);
        assert_eq!(lanes.rows_for(2048), 2048);
        for slot in 0..8 {
            assert_eq!(lanes.split(slot), (slot, 0));
        }
    }

    #[test]
    fn slots_walk_lanes_before_rows() {
        let lanes = BinaryLanes::new(3);
        assert_eq!(lanes.slots(1024), 3072);
        let walked: Vec<(usize, usize)> = (0..7).map(|s| lanes.split(s)).collect();
        assert_eq!(walked, vec![(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (2, 0)]);
    }

    /// The odd lane counts are the ones the add-hi airs actually use, and the ones a power-of-two
    /// mapping would have got wrong.
    #[test]
    fn split_matches_div_and_rem_for_every_lane_count_in_use() {
        for &n in &[1usize, 2, 3, 4, 5, 6, 9] {
            let lanes = BinaryLanes::new(n);
            for slot in 0..(4 * n + 3) {
                assert_eq!(lanes.split(slot), (slot / n, slot % n), "lanes={n} slot={slot}");
            }
        }
    }

    #[test]
    fn rows_for_pads_the_last_row() {
        let lanes = BinaryLanes::new(5);
        assert_eq!(lanes.rows_for(0), 0);
        assert_eq!(lanes.rows_for(1), 1, "one operation still takes a whole row");
        assert_eq!(lanes.rows_for(5), 1);
        assert_eq!(lanes.rows_for(6), 2);
    }

    #[test]
    #[should_panic(expected = "greater than 0")]
    fn rejects_zero_lanes() {
        BinaryLanes::new(0);
    }
}
