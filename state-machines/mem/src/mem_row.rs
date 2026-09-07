//! Binds a `Mem` row type to the air it fills.
//!
//! `Mem`, `MemLarge` and `MemHuge` commit the same columns at three widths (`addr[1]` vs
//! `addr[4]` vs `addr[8]`), so the pil-helpers give each its own row type, its own row-ops trait,
//! its own trace alias and its own air-values type. None of that is interchangeable, yet the
//! witness that fills them is one algorithm.
//!
//! [`MemRow`] is what makes it one: the state machine is generic over a row that implements it,
//! and `Self::LANES_X_ROW` tells the shared fill how many slots ride on a row. The setters keep
//! the generated names on purpose — the fill is bound by this trait alone, so there is nothing to
//! be ambiguous with, and the body reads as it did before three airs existed.
//!
//! [`MemSegmentValues`] is the same idea for the air values: the three types are declared
//! identically, so the fill collects the segment's values once in primitive form and each air's
//! impl copies them into its own type.

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use zisk_pil::{
    MemAirValues, MemHugeAirValues, MemHugeTrace, MemHugeTraceRowOps, MemLargeAirValues,
    MemLargeTrace, MemLargeTraceRowOps, MemTrace, MemTraceRowOps,
};
use zisk_sm_mem_common::lanes_x_row;

/// The air values one `Mem` segment ends up with, before they are written into the air-values
/// type of whichever air is being proved.
#[derive(Debug, Default, Clone, Copy)]
pub struct MemSegmentValues {
    pub segment_id: usize,
    pub is_first_segment: bool,
    pub is_last_segment: bool,
    pub previous_segment_step: u64,
    pub previous_segment_addr: u32,
    pub previous_segment_value: u64,
    pub segment_last_step: u64,
    pub segment_last_addr: u32,
    pub segment_last_value: [u32; 2],
    pub distance_base: [u16; 2],
    pub distance_end: [u16; 2],
}

/// Ties a `Mem` row type to the trace of the air it fills and to that air's packing width.
pub trait MemRow<F: PrimeField64, T>: Default + Copy + Send + Sync {
    /// Memory lanes this air packs into one row.
    const LANES_X_ROW: usize;

    fn set_addr(&mut self, lane: usize, value: u32);
    fn get_addr(&self, lane: usize) -> u32;
    fn set_step(&mut self, lane: usize, value: u64);
    fn get_step(&self, lane: usize) -> u64;
    fn set_sel(&mut self, lane: usize, value: bool);
    fn set_addr_changes(&mut self, lane: usize, value: bool);
    fn set_step_dual(&mut self, lane: usize, value: u64);
    fn get_step_dual(&self, lane: usize) -> u64;
    fn set_sel_dual(&mut self, lane: usize, value: bool);
    fn get_sel_dual(&self, lane: usize) -> bool;
    fn set_value(&mut self, lane: usize, index: usize, value: u32);
    fn get_value(&self, lane: usize, index: usize) -> u32;
    fn set_wr(&mut self, lane: usize, value: bool);
    fn get_wr(&self, lane: usize) -> bool;
    fn set_previous_step(&mut self, lane: usize, value: u64);
    fn set_l_increment(&mut self, lane: usize, value: u32);
    fn get_l_increment(&self, lane: usize) -> u32;
    fn set_h_increment(&mut self, lane: usize, value: u16);
    fn get_h_increment(&self, lane: usize) -> u16;
    fn set_read_same_addr(&mut self, lane: usize, value: bool);

    /// This air's generated air-values type.
    type AirValues;

    fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<T>;
    fn new_trace_zeroes(trace_buffer: Vec<F>) -> ProofmanResult<T>;
    fn trace_num_rows(trace: &T) -> usize;
    fn trace_rows_mut(trace: &mut T) -> &mut [Self];

    /// This air's air values, filled from `values`.
    ///
    /// Split out of [`Self::into_air_instance`] so the mapping can be checked without
    /// allocating a whole trace — see the tests at the bottom of this file.
    fn air_values(values: &MemSegmentValues) -> Self::AirValues;

    /// Wraps the filled trace into an `AirInstance` carrying this air's air values.
    fn into_air_instance(trace: &mut T, values: &MemSegmentValues) -> AirInstance<F>;
}

/// Emits the row-to-trace binding for one `Mem` air. The bodies are identical; only the row-ops
/// trait, the trace alias, its air values and the packing width change.
macro_rules! impl_mem_row {
    ($row_ops:ident, $trace:ident, $air_values:ident, $lanes:expr) => {
        impl<F: PrimeField64, R: $row_ops<F>> MemRow<F, $trace<R>> for R {
            const LANES_X_ROW: usize = $lanes;

            #[inline(always)]
            fn set_addr(&mut self, lane: usize, value: u32) {
                $row_ops::set_addr(self, lane, value);
            }
            #[inline(always)]
            fn get_addr(&self, lane: usize) -> u32 {
                $row_ops::get_addr(self, lane)
            }
            #[inline(always)]
            fn set_step(&mut self, lane: usize, value: u64) {
                $row_ops::set_step(self, lane, value);
            }
            #[inline(always)]
            fn get_step(&self, lane: usize) -> u64 {
                $row_ops::get_step(self, lane)
            }
            #[inline(always)]
            fn set_sel(&mut self, lane: usize, value: bool) {
                $row_ops::set_sel(self, lane, value);
            }
            #[inline(always)]
            fn set_addr_changes(&mut self, lane: usize, value: bool) {
                $row_ops::set_addr_changes(self, lane, value);
            }
            #[inline(always)]
            fn set_step_dual(&mut self, lane: usize, value: u64) {
                $row_ops::set_step_dual(self, lane, value);
            }
            #[inline(always)]
            fn get_step_dual(&self, lane: usize) -> u64 {
                $row_ops::get_step_dual(self, lane)
            }
            #[inline(always)]
            fn set_sel_dual(&mut self, lane: usize, value: bool) {
                $row_ops::set_sel_dual(self, lane, value);
            }
            #[inline(always)]
            fn get_sel_dual(&self, lane: usize) -> bool {
                $row_ops::get_sel_dual(self, lane)
            }
            #[inline(always)]
            fn set_value(&mut self, lane: usize, index: usize, value: u32) {
                $row_ops::set_value(self, lane, index, value);
            }
            #[inline(always)]
            fn get_value(&self, lane: usize, index: usize) -> u32 {
                $row_ops::get_value(self, lane, index)
            }
            #[inline(always)]
            fn set_wr(&mut self, lane: usize, value: bool) {
                $row_ops::set_wr(self, lane, value);
            }
            #[inline(always)]
            fn get_wr(&self, lane: usize) -> bool {
                $row_ops::get_wr(self, lane)
            }
            #[inline(always)]
            fn set_previous_step(&mut self, lane: usize, value: u64) {
                $row_ops::set_previous_step(self, lane, value);
            }
            #[inline(always)]
            fn set_l_increment(&mut self, lane: usize, value: u32) {
                $row_ops::set_l_increment(self, lane, value);
            }
            #[inline(always)]
            fn get_l_increment(&self, lane: usize) -> u32 {
                $row_ops::get_l_increment(self, lane)
            }
            #[inline(always)]
            fn set_h_increment(&mut self, lane: usize, value: u16) {
                $row_ops::set_h_increment(self, lane, value);
            }
            #[inline(always)]
            fn get_h_increment(&self, lane: usize) -> u16 {
                $row_ops::get_h_increment(self, lane)
            }
            #[inline(always)]
            fn set_read_same_addr(&mut self, lane: usize, value: bool) {
                $row_ops::set_read_same_addr(self, lane, value);
            }

            fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<$trace<R>> {
                $trace::<R>::new_from_vec(trace_buffer)
            }

            fn new_trace_zeroes(trace_buffer: Vec<F>) -> ProofmanResult<$trace<R>> {
                $trace::<R>::new_from_vec_zeroes(trace_buffer)
            }

            fn trace_num_rows(trace: &$trace<R>) -> usize {
                trace.num_rows()
            }

            fn trace_rows_mut(trace: &mut $trace<R>) -> &mut [Self] {
                &mut trace.buffer
            }

            type AirValues = $air_values<'static, F>;

            fn air_values(values: &MemSegmentValues) -> Self::AirValues {
                let mut air_values = $air_values::<F>::new();
                air_values.segment_id = F::from_usize(values.segment_id);
                air_values.is_first_segment = F::from_bool(values.is_first_segment);
                air_values.is_last_segment = F::from_bool(values.is_last_segment);
                air_values.previous_segment_step = F::from_u64(values.previous_segment_step);
                air_values.previous_segment_addr = F::from_u32(values.previous_segment_addr);
                air_values.previous_segment_value[0] =
                    F::from_u32(values.previous_segment_value as u32);
                air_values.previous_segment_value[1] =
                    F::from_u32((values.previous_segment_value >> 32) as u32);
                air_values.segment_last_step = F::from_u64(values.segment_last_step);
                air_values.segment_last_addr = F::from_u32(values.segment_last_addr);
                air_values.segment_last_value[0] = F::from_u32(values.segment_last_value[0]);
                air_values.segment_last_value[1] = F::from_u32(values.segment_last_value[1]);
                air_values.distance_base[0] = F::from_u16(values.distance_base[0]);
                air_values.distance_base[1] = F::from_u16(values.distance_base[1]);
                air_values.distance_end[0] = F::from_u16(values.distance_end[0]);
                air_values.distance_end[1] = F::from_u16(values.distance_end[1]);
                air_values
            }

            fn into_air_instance(
                trace: &mut $trace<R>,
                values: &MemSegmentValues,
            ) -> AirInstance<F> {
                let mut air_values = <Self as MemRow<F, $trace<R>>>::air_values(values);
                AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
            }
        }
    };
}

impl_mem_row!(MemTraceRowOps, MemTrace, MemAirValues, lanes_x_row::MEM);
impl_mem_row!(MemLargeTraceRowOps, MemLargeTrace, MemLargeAirValues, lanes_x_row::MEM_LARGE);
impl_mem_row!(MemHugeTraceRowOps, MemHugeTrace, MemHugeAirValues, lanes_x_row::MEM_HUGE);

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::{Field, Goldilocks, PrimeField64};
    use zisk_pil::{MemHugeTraceRow, MemLargeTraceRow, MemTraceRow};

    /// `lanes_x_row` pins the constants against the generated ROWS; this pins the STATE MACHINE
    /// against the constants, which is the other half — a width that does not match the row it
    /// fills would index past the end of a lane array and only show up as a panic deep inside
    /// witness computation.
    #[test]
    fn the_state_machine_packs_what_its_air_holds() {
        type F = Goldilocks;

        macro_rules! check {
            ($row:ident, $trace:ident, $konst:path) => {
                assert_eq!(
                    <$row<F> as MemRow<F, $trace<$row<F>>>>::LANES_X_ROW,
                    $row::<F>::default().addr.len(),
                    concat!(stringify!($row), " packs a different width than its air"),
                );
                assert_eq!(
                    <$row<F> as MemRow<F, $trace<$row<F>>>>::LANES_X_ROW,
                    $konst,
                    concat!(stringify!($row), " does not use ", stringify!($konst)),
                );
            };
        }

        check!(MemTraceRow, MemTrace, lanes_x_row::MEM);
        check!(MemLargeTraceRow, MemLargeTrace, lanes_x_row::MEM_LARGE);
        check!(MemHugeTraceRow, MemHugeTrace, lanes_x_row::MEM_HUGE);
    }

    /// Values distinct from each other and all non-zero, so a field left unwritten shows up as a
    /// zero and a field written from the wrong source shows up as the wrong number.
    fn sample_values() -> MemSegmentValues {
        MemSegmentValues {
            segment_id: 7,
            is_first_segment: true,
            is_last_segment: true,
            previous_segment_step: 0x0000_00AB_CDEF_1234,
            previous_segment_addr: 0x1122_3344,
            previous_segment_value: 0x8899_AABB_CCDD_EEFF,
            segment_last_step: 0x0000_0055_6677_8899,
            segment_last_addr: 0x5566_7788,
            segment_last_value: [0x1234_5678, 0x9ABC_DEF0],
            distance_base: [0x1111, 0x2222],
            distance_end: [0x3333, 0x4444],
        }
    }

    /// Every air value the segment carries must reach the buffer. A field the fill forgets stays
    /// at the zero `MemAirValues::new` left behind, which is a legal-looking value the prover
    /// would happily commit to — so check them one by one rather than trusting the mapping.
    #[test]
    fn every_air_value_reaches_the_buffer() {
        type F = Goldilocks;
        let v = sample_values();
        let av = <MemTraceRow<F> as MemRow<F, MemTrace<MemTraceRow<F>>>>::air_values(&v);

        assert_eq!(av.segment_id, F::from_usize(v.segment_id));
        assert_eq!(av.is_first_segment, F::from_bool(v.is_first_segment));
        assert_eq!(av.is_last_segment, F::from_bool(v.is_last_segment));
        assert_eq!(av.previous_segment_step, F::from_u64(v.previous_segment_step));
        assert_eq!(av.previous_segment_addr, F::from_u32(v.previous_segment_addr));
        assert_eq!(av.previous_segment_value[0], F::from_u32(v.previous_segment_value as u32));
        assert_eq!(
            av.previous_segment_value[1],
            F::from_u32((v.previous_segment_value >> 32) as u32)
        );
        assert_eq!(av.segment_last_step, F::from_u64(v.segment_last_step));
        assert_eq!(av.segment_last_addr, F::from_u32(v.segment_last_addr));
        assert_eq!(av.segment_last_value[0], F::from_u32(v.segment_last_value[0]));
        assert_eq!(av.segment_last_value[1], F::from_u32(v.segment_last_value[1]));
        assert_eq!(av.distance_base[0], F::from_u16(v.distance_base[0]));
        assert_eq!(av.distance_base[1], F::from_u16(v.distance_base[1]));
        assert_eq!(av.distance_end[0], F::from_u16(v.distance_end[0]));
        assert_eq!(av.distance_end[1], F::from_u16(v.distance_end[1]));
    }

    /// The only slots the fill leaves at zero are `im_direct`, which proofman computes. Counting
    /// them is what catches a field that the per-air impls stopped writing: it would show up as
    /// one more zero than the intermediates account for.
    #[test]
    fn the_only_untouched_air_values_are_the_intermediates() {
        type F = Goldilocks;
        let v = sample_values();

        let buffers = [
            <MemTraceRow<F> as MemRow<F, MemTrace<MemTraceRow<F>>>>::air_values(&v).buffer,
            <MemLargeTraceRow<F> as MemRow<F, MemLargeTrace<MemLargeTraceRow<F>>>>::air_values(&v)
                .buffer,
            <MemHugeTraceRow<F> as MemRow<F, MemHugeTrace<MemHugeTraceRow<F>>>>::air_values(&v)
                .buffer,
        ];

        // `im_direct: [FieldExtension<F>; 6]` — six extension elements of three field elements.
        const IM_DIRECT_SLOTS: usize = 6 * 3;
        for (air, buffer) in buffers.iter().enumerate() {
            let zeros = buffer.iter().filter(|f| **f == F::ZERO).count();
            assert_eq!(
                zeros, IM_DIRECT_SLOTS,
                "air {air}: {zeros} air-value slots left at zero, expected only the {IM_DIRECT_SLOTS} intermediates"
            );
            // The intermediates are the tail of the `#[repr(C)]` declaration.
            assert!(
                buffer[buffer.len() - IM_DIRECT_SLOTS..].iter().all(|f| *f == F::ZERO),
                "air {air}: the zeros are not the trailing intermediates"
            );
        }

        // The three airs declare the same air values, which is what lets one `MemSegmentValues`
        // feed all of them; identical buffers is that assumption, checked.
        assert_eq!(buffers[0], buffers[1]);
        assert_eq!(buffers[1], buffers[2]);
    }
}
