use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use rayon::prelude::*;

use crate::{MemAlignBytesBlockRow, MemAlignInput};
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use zisk_pil::{
    MemAlignByteAirValues, MemAlignByteLargeAirValues, MemAlignByteLargeTrace, MemAlignByteTrace,
    MemAlignByteTraceRowOps, MemAlignReadByteAirValues, MemAlignReadByteLargeAirValues,
    MemAlignReadByteLargeTrace, MemAlignReadByteTrace, MemAlignReadByteTraceRowOps,
    MemAlignWriteByteAirValues, MemAlignWriteByteTrace, MemAlignWriteByteTraceRowOps,
    DUAL_RANGE_BYTE_ID,
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
    fn create_trace(trace_buffer: Vec<F>) -> ProofmanResult<T>;
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

// Implement the common trait for all trace types.
//
// Each of the read-write and read-only airs comes in two heights — `MemAlignByte` /
// `MemAlignByteLarge` and `MemAlignReadByte` / `MemAlignReadByteLarge` — that commit exactly the same
// columns, so they share the row type and only the trace alias (and with it `NUM_ROWS` and `AIR_ID`)
// differs. The bodies are emitted once per air by the macros below.

/// The trace lifecycle, identical for every air of the family.
macro_rules! impl_trace_lifecycle {
    ($trace:ident, $air_values:ident, $name:literal) => {
        fn create_trace(trace_buffer: Vec<F>) -> ProofmanResult<$trace<R>> {
            $trace::<R>::new_from_vec(trace_buffer)
        }
        fn get_num_rows(trace: &$trace<R>) -> usize {
            trace.num_rows()
        }
        fn name() -> &'static str {
            $name
        }
        fn get_row_mut(trace: &mut $trace<R>, index: usize) -> &mut Self {
            &mut trace[index]
        }
        fn create_instance_from_trace(trace: &mut $trace<R>, padding_row: usize) -> AirInstance<F> {
            let num_rows = trace.num_rows();
            let padding_size = num_rows - padding_row;
            if padding_size > 0 {
                let padding = trace[padding_row];
                trace.buffer[padding_row + 1..num_rows]
                    .par_iter_mut()
                    .for_each(|slot| *slot = padding);
            }
            let mut air_values = $air_values::<F>::new();
            air_values.padding_size = F::from_usize(padding_size);
            AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
        }
    };
}

/// The columns every air of the family commits.
macro_rules! impl_common_fields {
    () => {
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
            self.set_sel_high_4b(sel_high_4b);
            self.set_sel_high_2b(sel_high_2b);
            self.set_sel_high_b(sel_high_b);
            self.set_direct_value(direct_value);
            self.set_composed_value(composed_value);
            self.set_value_16b(value_16b);
            self.set_value_8b(value_8b);
            self.set_byte_value(byte_value);
            self.set_addr_w(addr_w);
            self.set_step(step);
        }
    };
}

/// One read-write air: it proves both directions and carries the `is_write` selector.
macro_rules! impl_read_write_air {
    ($trace:ident, $air_values:ident, $name:literal) => {
        impl<F: PrimeField64, R: MemAlignByteTraceRowOps<F>> MemAlignByteRow<F, $trace<R>> for R {
            impl_common_fields!();

            #[inline(always)]
            fn set_write_fields(
                &mut self,
                is_write: bool,
                written_composed_value: u32,
                written_byte_value: u8,
                mem_write_values: [u32; 2],
            ) {
                self.set_is_write(is_write);
                self.set_written_composed_value(written_composed_value);
                self.set_written_byte_value(written_byte_value);
                self.set_bus_byte(if is_write {
                    self.get_written_byte_value()
                } else {
                    self.get_byte_value()
                });
                self.set_all_mem_write_values(&mem_write_values);
            }
            #[inline(always)]
            fn valid_for_read() -> bool {
                true
            }
            #[inline(always)]
            fn valid_for_write() -> bool {
                true
            }

            impl_trace_lifecycle!($trace, $air_values, $name);
        }
    };
}

/// One read-only air: the write columns are absent, so setting them is a no-op.
macro_rules! impl_read_air {
    ($trace:ident, $air_values:ident, $name:literal) => {
        impl<F: PrimeField64, R: MemAlignReadByteTraceRowOps<F>> MemAlignByteRow<F, $trace<R>>
            for R
        {
            impl_common_fields!();

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

            impl_trace_lifecycle!($trace, $air_values, $name);
        }
    };
}

impl_read_write_air!(MemAlignByteTrace, MemAlignByteAirValues, "MemAlignByteTrace");
impl_read_write_air!(MemAlignByteLargeTrace, MemAlignByteLargeAirValues, "MemAlignByteLargeTrace");

impl_read_air!(MemAlignReadByteTrace, MemAlignReadByteAirValues, "MemAlignReadByteTrace");
impl_read_air!(
    MemAlignReadByteLargeTrace,
    MemAlignReadByteLargeAirValues,
    "MemAlignReadByteLargeTrace"
);

// The write-only air has no `Large` sibling, so it is written out directly.
impl<F: PrimeField64, R: MemAlignWriteByteTraceRowOps<F>>
    MemAlignByteRow<F, MemAlignWriteByteTrace<R>> for R
{
    impl_common_fields!();

    #[inline(always)]
    fn set_write_fields(
        &mut self,
        _is_write: bool,
        written_composed_value: u32,
        written_byte_value: u8,
        mem_write_values: [u32; 2],
    ) {
        self.set_written_composed_value(written_composed_value);
        self.set_written_byte_value(written_byte_value);
        self.set_all_mem_write_values(&mem_write_values);
    }
    #[inline(always)]
    fn valid_for_read() -> bool {
        false
    }
    #[inline(always)]
    fn valid_for_write() -> bool {
        true
    }

    impl_trace_lifecycle!(
        MemAlignWriteByteTrace,
        MemAlignWriteByteAirValues,
        "MemAlignWriteByteTrace"
    );
}

const OFFSET_MASK: u32 = 0x07;
const OFFSET_BITS: u32 = 3;

pub struct MemAlignByteSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

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
            table_dual_byte_id: std
                .get_virtual_table_id(DUAL_RANGE_BYTE_ID)
                .expect("Failed to get dual byte table ID"),
            table_16b_id: std.get_range_id(0, 0xFFFF, None).expect("Failed to get 16b table ID"),
            table_8b_id: std.get_range_id(0, 0xFF, None).expect("Failed to get 8b table ID"),
        })
    }

    pub fn compute_witness<T, R: MemAlignByteRow<F, T>>(
        &self,
        mem_ops: &[Vec<MemAlignInput>],
        used_rows: usize,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = R::create_trace(trace_buffer)?;
        let num_rows = R::get_num_rows(&trace);

        tracing::debug!(
            "··· Creating {} instance [{} / {} rows filled {:.2}%]",
            R::name(),
            used_rows,
            num_rows,
            used_rows as f64 / num_rows as f64 * 100.0
        );

        let mut dual_mults = vec![0u64; 65536];
        let mut mults_16b = vec![0u32; 65536];
        let mut mults_8b = vec![0u32; 256];

        let mut irow = 0;
        for inner_memp_ops in mem_ops.iter() {
            for input in inner_memp_ops.iter() {
                assert!(irow < num_rows);
                self.compute_row_witness(
                    input,
                    irow,
                    R::get_row_mut(&mut trace, irow),
                    &mut dual_mults,
                    &mut mults_16b,
                    &mut mults_8b,
                );
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
                &mut dual_mults,
                &mut mults_16b,
                &mut mults_8b,
            );
            dual_mults[0] += padding_size - 1;
            mults_16b[0] += (padding_size - 1) as u32;
            if R::valid_for_write() {
                mults_8b[0] += (padding_size - 1) as u32;
            }
        }

        self.std.inc_virtual_rows_ranged(self.table_dual_byte_id, None, &dual_mults);
        self.std.range_check_ranged(self.table_16b_id, None, &mults_16b);
        if R::valid_for_write() {
            self.std.range_check_ranged(self.table_8b_id, None, &mults_8b);
        }

        Ok(R::create_instance_from_trace(&mut trace, irow))
    }

    /// Computes one row and writes it, updating the multiplicities of the ranges it uses.
    #[allow(clippy::too_many_arguments)]
    fn compute_row_witness<T, R: MemAlignByteRow<F, T>>(
        &self,
        input: &MemAlignInput,
        irow: usize,
        row: &mut R,
        dual_mults: &mut [u64],
        mults_16b: &mut [u32],
        mults_8b: &mut [u32],
    ) {
        let values = byte_row_values(input);

        row.set_common_fields(
            values.sel_high_4b,
            values.sel_high_2b,
            values.sel_high_b,
            values.direct_value,
            values.composed_value,
            values.value_16b,
            values.value_8b,
            values.byte_value,
            values.addr_w,
            values.step,
        );
        row.set_write_fields(
            values.is_write,
            values.written_composed_value,
            values.written_byte_value,
            values.mem_write_values,
        );

        add_byte_row_mults(&values, R::valid_for_write(), dual_mults, mults_16b, mults_8b);

        if input.is_write {
            assert!(
                R::valid_for_write(),
                "Row type does not support write operations ({}) row:{irow} step:{}",
                std::any::type_name::<R>(),
                values.step,
            );
        } else {
            assert!(
                R::valid_for_read(),
                "Row type does not support read operations ({}) row:{irow} step:{}",
                std::any::type_name::<R>(),
                values.step,
            );
        }
    }

    /// Fills the `bytes_` block of a `CompactMemAlign` trace and raises the multiplicities of the
    /// ranges its rows are checked against.
    ///
    /// See [`fill_bytes_block_rows`] for what the fill does. Returns the virtual rows the
    /// operations took; what is left of the block is its padding, which is the air value the bus
    /// needs to cancel what the padding lanes send to it.
    pub(crate) fn fill_bytes_block<R: MemAlignBytesBlockRow<F>>(
        &self,
        ops: &[&MemAlignInput],
        rows: &mut [R],
    ) -> usize {
        let (used, mults) = fill_bytes_block_rows(ops, rows);

        self.std.inc_virtual_rows_ranged(self.table_dual_byte_id, None, &mults.dual);
        self.std.range_check_ranged(self.table_16b_id, None, &mults.value_16b);
        self.std.range_check_ranged(self.table_8b_id, None, &mults.value_8b);

        used
    }
}

/// Fills the `bytes_` block of a `CompactMemAlign` trace: one operation per virtual row, in the
/// order they were collected, then padding to the end of the block.
///
/// `rows` is the whole fused trace, and this only ever touches the `bytes_*` columns -- the `full_`
/// fill writes the others over the same rows.
///
/// The padding row is the one the standalone air pads with: the zero operation, whose columns are
/// all zero. It is written out rather than left alone because the trace buffer comes from the
/// recycled pool and is not zeroed.
pub(crate) fn fill_bytes_block_rows<F: PrimeField64, R: MemAlignBytesBlockRow<F>>(
    ops: &[&MemAlignInput],
    rows: &mut [R],
) -> (usize, ByteMults) {
    let lanes = R::bytes_lanes_x_row();
    let num_slots = rows.len() * lanes;
    assert!(
        ops.len() <= num_slots,
        "CompactMemAlign bytes block: {} operations do not fit in {num_slots} virtual rows",
        ops.len()
    );

    tracing::debug!(
        "··· Filling CompactMemAlign bytes block [{} / {} virtual rows filled {:.2}%]",
        ops.len(),
        num_slots,
        ops.len() as f64 / num_slots as f64 * 100.0
    );

    // One group per thread of the pool this witness computation is already running in. Byte rows
    // are independent of each other, so any cut is a valid one.
    let n_ranges = rayon::current_num_threads().max(1);
    let rows_x_group = rows.len().div_ceil(n_ranges).max(1);

    let mults = rows
        .par_chunks_mut(rows_x_group)
        .enumerate()
        .map(|(group, chunk)| {
            let mut mults = ByteMults::new();
            let base = group * rows_x_group * lanes;
            for index in 0..(chunk.len() * lanes) {
                let slot = base + index;
                let values = if slot < ops.len() {
                    byte_row_values(ops[slot])
                } else {
                    ByteRowValues::default()
                };
                chunk[index / lanes].set_bytes_lane(index % lanes, &values);
                add_byte_row_mults(
                    &values,
                    true,
                    &mut mults.dual,
                    &mut mults.value_16b,
                    &mut mults.value_8b,
                );
            }
            mults
        })
        .reduce(ByteMults::new, ByteMults::merge);

    (ops.len(), mults)
}

/// The range multiplicities one group of byte rows raises, so the groups can be filled in parallel
/// and added up at the end.
pub(crate) struct ByteMults {
    dual: Vec<u64>,
    value_16b: Vec<u32>,
    value_8b: Vec<u32>,
}

impl ByteMults {
    pub(crate) fn new() -> Self {
        Self { dual: vec![0u64; 65536], value_16b: vec![0u32; 65536], value_8b: vec![0u32; 256] }
    }

    fn merge(mut self, other: Self) -> Self {
        for (a, b) in self.dual.iter_mut().zip(other.dual) {
            *a += b;
        }
        for (a, b) in self.value_16b.iter_mut().zip(other.value_16b) {
            *a += b;
        }
        for (a, b) in self.value_8b.iter_mut().zip(other.value_8b) {
            *a += b;
        }
        self
    }
}

/// Every column of one byte row.
///
/// The computation is pure -- the offset of the address decides all of it -- so it is written once
/// here and used by the two fills: the one of the standalone airs above and the one of the
/// `bytes_` block of `CompactMemAlign`, which writes the same columns one lane at a time.
#[derive(Clone, Copy, Default)]
pub(crate) struct ByteRowValues {
    pub sel_high_4b: bool,
    pub sel_high_2b: bool,
    pub sel_high_b: bool,
    pub direct_value: u32,
    pub composed_value: u32,
    pub value_16b: u16,
    pub value_8b: u8,
    pub byte_value: u8,
    pub addr_w: u32,
    pub step: u64,
    pub is_write: bool,
    pub written_composed_value: u32,
    pub written_byte_value: u8,
    pub mem_write_values: [u32; 2],
}

/// The columns one memory operation takes in a byte row.
pub(crate) fn byte_row_values(input: &MemAlignInput) -> ByteRowValues {
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

    ByteRowValues {
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
        is_write: input.is_write,
        written_composed_value,
        written_byte_value,
        mem_write_values: write_values,
    }
}

/// Adds one row to the multiplicities of the ranges its columns are checked against.
///
/// `with_write` is the air's `valid_for_write`: the 8-bit range is only raised by the airs that
/// commit `written_byte_value`.
pub(crate) fn add_byte_row_mults(
    values: &ByteRowValues,
    with_write: bool,
    dual_mults: &mut [u64],
    mults_16b: &mut [u32],
    mults_8b: &mut [u32],
) {
    dual_mults[(values.value_8b as u16 + ((values.byte_value as u16) << 8)) as usize] += 1;
    mults_16b[values.value_16b as usize] += 1;
    if with_write {
        mults_8b[values.written_byte_value as usize] += 1;
    }
}
