use std::sync::Arc;

#[cfg(feature = "debug_mem_align")]
use std::sync::Mutex;

use fields::PrimeField64;
use pil_std_lib::Std;
use rayon::prelude::*;

use crate::MemAlignInput;
use proofman_common::{AirInstance, FromTrace};
use zisk_pil::{
    MemAlignByteAirValues, MemAlignByteTrace, MemAlignByteTraceRow, MemAlignReadByteAirValues,
    MemAlignReadByteTrace, MemAlignReadByteTraceRow, MemAlignWriteByteAirValues,
    MemAlignWriteByteTrace, MemAlignWriteByteTraceRow,
};

pub trait MemAlignByteRow<F: PrimeField64, T> {
    #[allow(clippy::too_many_arguments)]
    fn set_common_fields(
        &mut self,
        sel_high_4b: bool,
        sel_high_2b: bool,
        sel_high_b: bool,
        direct_value: u32,
        composed_value: u32,
        value_16b: u16,
        value_8b: u8,
        byte_value: u8,
        addr_w: u32,
        step: u64,
    );
    fn set_write_fields(
        &mut self,
        is_write: bool,
        written_composed_value: u32,
        written_byte_value: u8,
        mem_write_values: [u32; 2],
    );
    fn valid_for_read() -> bool;
    fn valid_for_write() -> bool;
    fn create_trace(trace_buffer: Vec<F>) -> T;
    fn get_num_rows(trace: &T) -> usize;
    fn name() -> &'static str;
    fn create_instance_from_trace(trace: &mut T, padding_row: usize) -> AirInstance<F>;
    fn get_row_mut(trace: &mut T, index: usize) -> &mut Self;
}

// Helper function to avoid code duplication in create_instance_from_trace
// fn create_instance_from_trace_helper<F: PrimeField64, T, V>(
//     trace: &mut T,
//     padding_row: usize,
//     mut air_values: V,
//     set_padding_size: impl FnOnce(&mut V, F),
// ) -> AirInstance<F>
// where
//     T: proofman_common::trace::Trace<F> + std::ops::Index<usize> + std::ops::IndexMut<usize>,
//     T::Output: Clone,
//     V: proofman_common::trace::Values<F>,
// {
//     let num_rows = trace.num_rows();
//     let padding = trace[padding_row].clone();
//     let padding_size = num_rows - padding_row;
//     set_padding_size(&mut air_values, F::from_usize(padding_size));
//     trace.row_slice_mut()[padding_row + 1..num_rows]
//         .par_iter_mut()
//         .for_each(|slot| *slot = padding);
//     AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
// }

// Implement the common trait for all trace types
impl<F: PrimeField64> MemAlignByteRow<F, MemAlignByteTrace<F>> for MemAlignByteTraceRow<F> {
    #[inline(always)]
    fn set_common_fields(
        &mut self,
        sel_high_4b: bool,
        sel_high_2b: bool,
        sel_high_b: bool,
        direct_value: u32,
        composed_value: u32,
        value_16b: u16,
        value_8b: u8,
        byte_value: u8,
        addr_w: u32,
        step: u64,
    ) {
        self.sel_high_4b = F::from_bool(sel_high_4b);
        self.sel_high_2b = F::from_bool(sel_high_2b);
        self.sel_high_b = F::from_bool(sel_high_b);
        self.direct_value = F::from_u32(direct_value);
        self.composed_value = F::from_u32(composed_value);
        self.value_16b = F::from_u16(value_16b);
        self.value_8b = F::from_u8(value_8b);
        self.byte_value = F::from_u8(byte_value);
        self.addr_w = F::from_u32(addr_w);
        self.step = F::from_u64(step);
    }
    #[inline(always)]
    fn set_write_fields(
        &mut self,
        is_write: bool,
        written_composed_value: u32,
        written_byte_value: u8,
        mem_write_values: [u32; 2],
    ) {
        self.is_write = F::from_bool(is_write);
        self.written_composed_value = F::from_u32(written_composed_value);
        self.written_byte_value = F::from_u8(written_byte_value);
        self.bus_byte = if is_write { self.written_byte_value } else { self.byte_value };
        self.mem_write_values =
            [F::from_u32(mem_write_values[0]), F::from_u32(mem_write_values[1])];
    }
    #[inline(always)]
    fn valid_for_read() -> bool {
        true
    }
    #[inline(always)]
    fn valid_for_write() -> bool {
        true
    }
    fn create_trace(trace_buffer: Vec<F>) -> MemAlignByteTrace<F> {
        MemAlignByteTrace::new_from_vec(trace_buffer)
    }
    fn get_num_rows(trace: &MemAlignByteTrace<F>) -> usize {
        trace.num_rows()
    }
    fn name() -> &'static str {
        "MemAlignByteTrace"
    }
    fn get_row_mut(trace: &mut MemAlignByteTrace<F>, index: usize) -> &mut Self {
        &mut trace[index]
    }
    fn create_instance_from_trace(
        trace: &mut MemAlignByteTrace<F>,
        padding_row: usize,
    ) -> AirInstance<F> {
        let num_rows = trace.num_rows();
        let padding_size = num_rows - padding_row;
        if padding_size > 0 {
            let padding = trace[padding_row];
            trace.row_slice_mut()[padding_row + 1..num_rows]
                .par_iter_mut()
                .for_each(|slot| *slot = padding);
        }
        let mut air_values = MemAlignByteAirValues::<F>::new();
        air_values.padding_size = F::from_usize(padding_size);
        AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
    }
    // fn create_instance_from_trace(
    //     trace: &mut MemAlignByteTrace<F>,
    //     padding_row: usize,
    // ) -> AirInstance<F> {
    //     create_instance_from_trace_helper(
    //         trace,
    //         padding_row,
    //         MemAlignByteAirValues::<F>::new(),
    //         |air_values, size| air_values.padding_size = size,
    //     )
    // }
}

impl<F: PrimeField64> MemAlignByteRow<F, MemAlignReadByteTrace<F>> for MemAlignReadByteTraceRow<F> {
    fn set_common_fields(
        &mut self,
        sel_high_4b: bool,
        sel_high_2b: bool,
        sel_high_b: bool,
        direct_value: u32,
        composed_value: u32,
        value_16b: u16,
        value_8b: u8,
        byte_value: u8,
        addr_w: u32,
        step: u64,
    ) {
        self.sel_high_4b = F::from_bool(sel_high_4b);
        self.sel_high_2b = F::from_bool(sel_high_2b);
        self.sel_high_b = F::from_bool(sel_high_b);
        self.direct_value = F::from_u32(direct_value);
        self.composed_value = F::from_u32(composed_value);
        self.value_16b = F::from_u16(value_16b);
        self.value_8b = F::from_u8(value_8b);
        self.byte_value = F::from_u8(byte_value);
        self.addr_w = F::from_u32(addr_w);
        self.step = F::from_u64(step);
    }
    #[inline(always)]
    fn set_write_fields(
        &mut self,
        _is_write: bool,
        _written_composed_value: u32,
        _written_byte_value: u8,
        _mem_write_values: [u32; 2],
    ) {
    }
    #[inline(always)]
    fn valid_for_read() -> bool {
        true
    }
    #[inline(always)]
    fn valid_for_write() -> bool {
        false
    }
    fn create_trace(trace_buffer: Vec<F>) -> MemAlignReadByteTrace<F> {
        MemAlignReadByteTrace::new_from_vec(trace_buffer)
    }
    fn get_num_rows(trace: &MemAlignReadByteTrace<F>) -> usize {
        trace.num_rows()
    }
    fn name() -> &'static str {
        "MemAlignReadByteTrace"
    }
    fn get_row_mut(trace: &mut MemAlignReadByteTrace<F>, index: usize) -> &mut Self {
        &mut trace[index]
    }
    fn create_instance_from_trace(
        trace: &mut MemAlignReadByteTrace<F>,
        padding_row: usize,
    ) -> AirInstance<F> {
        let num_rows = trace.num_rows();
        let padding_size = num_rows - padding_row;
        if padding_size > 0 {
            let padding = trace[padding_row];
            trace.row_slice_mut()[padding_row + 1..num_rows]
                .par_iter_mut()
                .for_each(|slot| *slot = padding);
        }
        let mut air_values = MemAlignReadByteAirValues::<F>::new();
        air_values.padding_size = F::from_usize(padding_size);
        AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
    }
    // fn create_instance_from_trace(
    //     trace: &mut MemAlignReadByteTrace<F>,
    //     padding_row: usize,
    // ) -> AirInstance<F> {
    //     create_instance_from_trace_helper(
    //         trace,
    //         padding_row,
    //         MemAlignReadByteAirValues::<F>::new(),
    //         |air_values, size| air_values.padding_size = size,
    //     )
    // }
}

impl<F: PrimeField64> MemAlignByteRow<F, MemAlignWriteByteTrace<F>>
    for MemAlignWriteByteTraceRow<F>
{
    fn set_common_fields(
        &mut self,
        sel_high_4b: bool,
        sel_high_2b: bool,
        sel_high_b: bool,
        direct_value: u32,
        composed_value: u32,
        value_16b: u16,
        value_8b: u8,
        byte_value: u8,
        addr_w: u32,
        step: u64,
    ) {
        self.sel_high_4b = F::from_bool(sel_high_4b);
        self.sel_high_2b = F::from_bool(sel_high_2b);
        self.sel_high_b = F::from_bool(sel_high_b);
        self.direct_value = F::from_u32(direct_value);
        self.composed_value = F::from_u32(composed_value);
        self.value_16b = F::from_u16(value_16b);
        self.value_8b = F::from_u8(value_8b);
        self.byte_value = F::from_u8(byte_value);
        self.addr_w = F::from_u32(addr_w);
        self.step = F::from_u64(step);
    }
    #[inline(always)]
    fn set_write_fields(
        &mut self,
        _is_write: bool,
        written_composed_value: u32,
        written_byte_value: u8,
        mem_write_values: [u32; 2],
    ) {
        self.written_composed_value = F::from_u32(written_composed_value);
        self.written_byte_value = F::from_u8(written_byte_value);
        self.mem_write_values =
            [F::from_u32(mem_write_values[0]), F::from_u32(mem_write_values[1])];
    }
    #[inline(always)]
    fn valid_for_read() -> bool {
        false
    }
    #[inline(always)]
    fn valid_for_write() -> bool {
        true
    }
    fn create_trace(trace_buffer: Vec<F>) -> MemAlignWriteByteTrace<F> {
        MemAlignWriteByteTrace::new_from_vec(trace_buffer)
    }
    fn get_num_rows(trace: &MemAlignWriteByteTrace<F>) -> usize {
        trace.num_rows()
    }

    fn name() -> &'static str {
        "MemAlignWriteByteTrace"
    }
    fn get_row_mut(trace: &mut MemAlignWriteByteTrace<F>, index: usize) -> &mut Self {
        &mut trace[index]
    }
    fn create_instance_from_trace(
        trace: &mut MemAlignWriteByteTrace<F>,
        padding_row: usize,
    ) -> AirInstance<F> {
        let num_rows = trace.num_rows();
        let padding_size = num_rows - padding_row;
        if padding_size > 0 {
            let padding = trace[padding_row];
            trace.row_slice_mut()[padding_row + 1..num_rows]
                .par_iter_mut()
                .for_each(|slot| *slot = padding);
        }
        let mut air_values = MemAlignWriteByteAirValues::<F>::new();
        air_values.padding_size = F::from_usize(padding_size);
        AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
    }
}

const OFFSET_MASK: u32 = 0x07;
const OFFSET_BITS: u32 = 3;
const DUAL_BYTE_TABLE_ID: usize = 88;

pub struct MemAlignByteSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    #[cfg(feature = "debug_mem_align")]
    num_computed_rows: Mutex<usize>,

    /// The table ID for the Mem Align ROM State Machine
    table_dual_byte_id: usize,

    table_16b_id: usize,
    table_8b_id: usize,
}

impl<F: PrimeField64> MemAlignByteSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Get the table ID
        Arc::new(Self {
            std: std.clone(),
            table_dual_byte_id: std.get_virtual_table_id(DUAL_BYTE_TABLE_ID),
            table_16b_id: std.get_range_id(0, 0xFFFF, None),
            table_8b_id: std.get_range_id(0, 0xFF, None),
            #[cfg(feature = "debug_mem_align")]
            num_computed_rows: Mutex::new(0),
        })
    }

    pub fn compute_witness<T, R: MemAlignByteRow<F, T>>(
        &self,
        mem_ops: &[Vec<MemAlignInput>],
        used_rows: usize,
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut trace = R::create_trace(trace_buffer);
        let num_rows = R::get_num_rows(&trace);

        tracing::info!(
            "··· Creating {} instance [{} / {} rows filled {:.2}%]",
            R::name(),
            used_rows,
            num_rows,
            used_rows as f64 / num_rows as f64 * 100.0
        );

        let mut irow = 0;
        for inner_memp_ops in mem_ops.iter() {
            for input in inner_memp_ops.iter() {
                assert!(irow < num_rows);
                self.compute_row_witness(input, irow, R::get_row_mut(&mut trace, irow));
                irow += 1;
            }
        }
        let padding_size = (num_rows - irow) as u64;
        if padding_size > 0 {
            let padding_row = R::get_row_mut(&mut trace, irow);
            self.compute_row_witness(
                &MemAlignInput {
                    addr: 0,
                    width: 1,
                    is_write: !R::valid_for_read(),
                    value: 0,
                    mem_values: [0, 0],
                    step: 0,
                },
                irow,
                padding_row,
            );
            // padding_size - 1, because compute_row_witness call range_check
            self.std.inc_virtual_row(self.table_dual_byte_id, 0, padding_size - 1);
            self.std.range_check(self.table_16b_id, 0, padding_size - 1);
            if R::valid_for_write() {
                self.std.range_check(self.table_8b_id, 0, padding_size - 1);
            }
        }
        R::create_instance_from_trace(&mut trace, irow)
    }

    /// Common logic for computing witness that can be shared across different trace types
    fn compute_row_witness<T, R: MemAlignByteRow<F, T>>(
        &self,
        input: &MemAlignInput,
        irow: usize,
        row: &mut R,
    ) {
        let addr = input.addr;

        let high_value = (input.mem_values[0] >> 32) as u32;
        let low_value = (input.mem_values[0] & 0xFFFF_FFFF) as u32;
        let offset = (addr & OFFSET_MASK) as u8;
        let addr_w = addr >> OFFSET_BITS;
        let step = input.step;

        let (
            sel_high_4b,
            sel_high_2b,
            sel_high_b,
            direct_value,
            composed_value,
            byte_value,
            value_16b,
            value_8b,
        ) = match offset {
            0 => (
                false,
                false,
                false,
                high_value,
                low_value,
                low_value as u8,
                (low_value >> 16) as u16,
                (low_value >> 8) as u8,
            ),
            1 => (
                false,
                false,
                true,
                high_value,
                low_value,
                (low_value >> 8) as u8,
                (low_value >> 16) as u16,
                low_value as u8,
            ),
            2 => (
                false,
                true,
                false,
                high_value,
                low_value,
                (low_value >> 16) as u8,
                low_value as u16,
                (low_value >> 24) as u8,
            ),
            3 => (
                false,
                true,
                true,
                high_value,
                low_value,
                (low_value >> 24) as u8,
                low_value as u16,
                (low_value >> 16) as u8,
            ),
            4 => (
                true,
                false,
                false,
                low_value,
                high_value,
                high_value as u8,
                (high_value >> 16) as u16,
                (high_value >> 8) as u8,
            ),
            5 => (
                true,
                false,
                true,
                low_value,
                high_value,
                (high_value >> 8) as u8,
                (high_value >> 16) as u16,
                high_value as u8,
            ),
            6 => (
                true,
                true,
                false,
                low_value,
                high_value,
                (high_value >> 16) as u8,
                high_value as u16,
                (high_value >> 24) as u8,
            ),
            7 => (
                true,
                true,
                true,
                low_value,
                high_value,
                (high_value >> 24) as u8,
                high_value as u16,
                (high_value >> 16) as u8,
            ),
            _ => unreachable!("Invalid offset"),
        };

        row.set_common_fields(
            sel_high_4b,
            sel_high_2b,
            sel_high_b,
            direct_value,
            composed_value,
            value_16b,
            value_8b,
            byte_value,
            addr_w,
            step,
        );
        self.std.inc_virtual_row(
            self.table_dual_byte_id,
            (value_8b as u16 + ((byte_value as u16) << 8)) as u64,
            1,
        );
        self.std.range_check(self.table_16b_id, value_16b as i64, 1);

        let written_byte_value = input.value as u8;
        let written_composed_value = match offset {
            0 => (low_value & 0xFFFF_FF00) | (written_byte_value as u32),
            1 => (low_value & 0xFFFF_00FF) | ((written_byte_value as u32) << 8),
            2 => (low_value & 0xFF00_FFFF) | ((written_byte_value as u32) << 16),
            3 => (low_value & 0x00FF_FFFF) | ((written_byte_value as u32) << 24),
            4 => (high_value & 0xFFFF_FF00) | (written_byte_value as u32),
            5 => (high_value & 0xFFFF_00FF) | ((written_byte_value as u32) << 8),
            6 => (high_value & 0xFF00_FFFF) | ((written_byte_value as u32) << 16),
            7 => (high_value & 0x00FF_FFFF) | ((written_byte_value as u32) << 24),
            _ => unreachable!("Invalid offset"),
        };
        let write_values = if offset < 4 {
            [written_composed_value, high_value]
        } else {
            [low_value, written_composed_value]
        };

        if R::valid_for_write() {
            self.std.range_check(self.table_8b_id, written_byte_value as i64, 1);
        }
        row.set_write_fields(
            input.is_write,
            written_composed_value,
            written_byte_value,
            write_values,
        );

        if input.is_write {
            assert!(
                R::valid_for_write(),
                "Row type does not support write operations ({}) row:{irow} step:{step}",
                std::any::type_name::<R>()
            );
        } else {
            assert!(
                R::valid_for_read(),
                "Row type does not support read operations ({}) row:{irow} step:{step}",
                std::any::type_name::<R>()
            );
        }
    }
}
