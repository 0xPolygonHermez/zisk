use std::sync::Arc;

use crate::{mem_sm::MemPreviousSegment, MemModule, MemOps};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::{
    fs::File,
    io::{BufWriter, Write},
};
use zisk_common::SegmentId;
use zisk_core::{ROM_ADDR, ROM_ADDR_MAX};
use zisk_pil::{
    RomDataAirValues, RomDataTrace, RomDataTraceRow, RomDataTraceRowOps, RomDataTraceRowPacked,
};
use zisk_sm_mem_common::{
    MemHelpers, MemLanes, MemModuleSegmentCheckPoint, RomDataLaneRow, MEMORY_INIT_STEP,
    MEM_BYTES_BITS,
};

pub const ROM_DATA_W_ADDR_INIT: u32 = ROM_ADDR as u32 >> MEM_BYTES_BITS;
pub const ROM_DATA_W_ADDR_END: u32 = ROM_ADDR_MAX as u32 >> MEM_BYTES_BITS;

const _: () = {
    assert!(ROM_ADDR_MAX <= 0xFFFF_FFFF, "ROM_DATA memory exceeds the 32-bit addressable range");
    assert!(
        (ROM_ADDR_MAX - ROM_ADDR) <= (128 << 20),
        "ROM_DATA is too large. ROM size must be <= 128MB"
    );
};

/// Lane layout of the `RomData` trace, read from the generated row so it always
/// follows `lanes_x_row` in `state-machines/mem/pil/rom_data.pil`.
#[inline]
fn lanes_of<F: PrimeField64, R: RomDataTraceRowOps<F>>() -> MemLanes {
    MemLanes::new(R::default().get_all_addr().len())
}

/// Same, for a row seen through [`RomDataLaneRow`]: the `RomData` row answers its own
/// `lanes_x_row`, the fused `CompactMem` row answers the one of its `rom_` block.
#[inline]
pub(crate) fn rom_data_lanes_of<F: PrimeField64, R: RomDataLaneRow<F>>() -> MemLanes {
    MemLanes::new(R::rom_data_lanes_x_row())
}

/// What the `RomData` fill produces besides the rows: the scalars its air values are built from.
pub(crate) struct RomDataFillOutput {
    /// Address and value of the last filled slot: what the padding repeats and what the segment
    /// hands to the next one.
    pub last_addr: u32,
    pub last_value: [u32; 2],
    /// Lanes the padding took, which is what the padding lookup subtracts from the bus.
    pub padding_size: usize,
}

pub struct RomDataSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    range_24bits_id: usize,
}

const OFFSET_USE_FLAG: u32 = 0x8000_0000;
const OFFSET_VALUE_MASK: u32 = 0x7FFF_FFFF;

#[allow(unused, unused_variables)]
impl<F: PrimeField64> RomDataSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_24bits_id =
            std.get_range_id(0, (1 << 24) - 1, None).expect("Failed to get 24 bits range ID");
        Arc::new(Self { range_24bits_id, std: std.clone() })
    }
    pub fn get_from_addr() -> u32 {
        ROM_DATA_W_ADDR_INIT
    }
    fn get_u32_values(&self, value: u64) -> (u32, u32) {
        (value as u32, (value >> 32) as u32)
    }
    pub fn get_to_addr() -> u32 {
        ROM_DATA_W_ADDR_END
    }

    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// `mem_ops` must be sorted by `(addr, step)` before this method is called.
    /// Rows are written sequentially: each operation is assigned the next
    /// available row in declaration order, so the trace is filled from top to
    /// bottom with no random-access indexing.
    ///
    /// Use this path when the GPU / planning stage is disabled
    /// (`legacy_mem_count_and_plan` feature flag) and the CPU planner provides
    /// pre-sorted inputs instead of offset tables.
    fn legacy_compute_witness(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.legacy_compute_witness_inner::<RomDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        } else {
            self.legacy_compute_witness_inner::<RomDataTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        }
    }
    fn legacy_compute_witness_inner<R: RomDataTraceRowOps<F>>(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = RomDataTrace::<R>::new_from_vec(trace_buffer)?;
        let lanes = lanes_of::<F, R>();
        let num_slots = lanes.slots(RomDataTrace::<R>::NUM_ROWS);
        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_slots,
            "RomDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_slots
        );

        // Force previous_segment_addr = 0 for first instance
        let previous_segment_addr: u32 = if segment_id == 0 { 0 } else { previous_segment.addr };
        let mut last_addr: u32 = previous_segment_addr;
        // `i` is a virtual row: `lanes.split(i)` gives the physical row and the lane inside it.
        let mut i = 0;

        for mem_op in mem_ops.iter() {
            let (row, lane) = lanes.split(i);
            trace[row].set_addr(lane, mem_op.addr);
            trace[row].set_step(lane, mem_op.step);

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[row].set_value(lane, 0, low_val);
            trace[row].set_value(lane, 1, high_val);

            let addr_change = last_addr != mem_op.addr;
            trace[row].set_addr_change(lane, addr_change || (i == 0 && segment_id == 0));

            last_addr = mem_op.addr;
            i += 1;
            if i >= num_slots {
                break;
            }
        }
        let count = i;

        let (last_row, last_lane) = lanes.split(count - 1);
        // The padding lanes repeat the last lane (same addr and value, addr_change = 0), so they
        // are sent to the bus as plain MEMORY_LOAD_OP and never as an INIT: the ROM provides
        // exactly one INIT per address, and that one was already consumed by the first access to
        // this address. The step is pinned to MEMORY_INIT_STEP because that is the step the
        // padding lookup subtracts (`mul: -padding_size` in rom_data.pil), so these extra proves
        // cancel out on the bus.
        let pad_addr = trace[last_row].get_addr(last_lane);
        let pad_value =
            [trace[last_row].get_value(last_lane, 0), trace[last_row].get_value(last_lane, 1)];
        for islot in count..num_slots {
            let (row, lane) = lanes.split(islot);
            trace[row].set_addr(lane, pad_addr);
            trace[row].set_step(lane, MEMORY_INIT_STEP);
            trace[row].set_value(lane, 0, pad_value[0]);
            trace[row].set_value(lane, 1, pad_value[1]);
            trace[row].set_addr_change(lane, false);
        }

        assert!(
            is_last_segment || count == num_slots,
            "All intermediate segments must fill all lanes"
        );

        let mut air_values = RomDataAirValues::<F>::new();
        let padding_size = num_slots - count;
        air_values.padding_size = F::from_u32(padding_size as u32);
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_addr = F::from_u32(previous_segment_addr);
        air_values.segment_last_addr = F::from_u32(last_addr);

        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);

        air_values.segment_last_value[0] = F::from_u32(pad_value[0]);
        air_values.segment_last_value[1] = F::from_u32(pad_value[1]);

        if is_last_segment {
            self.std.range_check_one(self.range_24bits_id, padding_size as u64);
        }

        #[cfg(feature = "debug_mem")]
        {
            let path = std::env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/rom_trace_{segment_id:04}.txt");
            Self::save_to_file(&trace, &filename);
        }

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_witness_with_offsets(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_with_offsets_inner::<RomDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
            )
        } else {
            self.compute_witness_with_offsets_inner::<RomDataTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
            )
        }
    }
    /// Fills a `RomData` segment's rows using a precomputed **offset table** (GPU path).
    ///
    /// Takes `rows` rather than a `RomDataTrace` because the fused `CompactMem` air carries this
    /// block inside a wider row (see [`RomDataLaneRow`]).
    ///
    /// `mem_ops` does not need to be sorted. Each operation is placed directly
    /// into the virtual row indicated by the `offsets` table, enabling
    /// random-access filling in a single pass.
    ///
    /// # Offset table layout
    ///
    /// The table is expressed in **virtual rows**: with `lanes_x_row` lanes per
    /// physical row, the virtual row `v` is the lane `v % lanes_x_row` of the
    /// physical row `v / lanes_x_row` (see [`MemLanes`]).
    ///
    /// `offset_base_addr` is the byte address of the first qword slot
    /// (i.e. the byte address of `offsets[0]`).  For every qword address
    /// `A = (offset_base_addr >> 3) + i` that falls inside this segment:
    ///
    /// * `offsets[i] = 0` — **halo slot**: address `A` belongs to the
    ///   previous segment (`previous_segment`).  Only slot 0 of a non-first
    ///   segment can be 0.
    /// * `offsets[i] = v + 1` — address `A` first appears at virtual row `v`
    ///   (1-based so that 0 is unambiguously the halo).
    /// * Addresses **absent** from this instance are forward-filled: the slot
    ///   for a missing address inherits the value of the *next* present
    ///   address's slot.  Consequently, when traversing `offsets` in ascending
    ///   index order, the first absent address is the one where
    ///   `offsets[i] == offsets[i + 1]` (no increment between consecutive
    ///   slots).
    pub(crate) fn fill_trace<R: RomDataLaneRow<F>>(
        &self,
        rows: &mut [R],
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        seg: &MemModuleSegmentCheckPoint,
    ) -> RomDataFillOutput {
        let lanes = rom_data_lanes_of::<F, R>();
        let num_slots = lanes.slots(rows.len());
        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_slots,
            "RomDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_slots
        );
        // `current_offsets` packs a 1-based virtual row plus a flag bit in a u32.
        debug_assert!(
            num_slots < OFFSET_USE_FLAG as usize,
            "RomDataSM: {num_slots} virtual rows do not fit in OFFSET_VALUE_MASK"
        );
        // save_offsets_to_file(
        //     seg,
        //     &format!("tmp/rom_data_trace_gpu_{segment_id:04}_offsets.txt"),
        // );
        let mut current_offsets = vec![0u32; seg.addr_range_slots as usize];

        #[cfg(debug_assertions)]
        let mut filled_slots = vec![false; num_slots];
        let offset_base_addr_w = seg.offsets_base_addr >> 3;

        if seg.offset_at(0) == 0 {
            current_offsets[0] = OFFSET_USE_FLAG;
            // first address == halo
        }

        for mem_op in mem_ops.iter() {
            let addr_index = (mem_op.addr - offset_base_addr_w) as usize;
            let addr_changes = current_offsets[addr_index] == 0;

            // `islot` is a virtual row: the offsets table is expressed in these units, so the
            // physical row and the lane inside it come from `lanes.split(islot)`.
            let islot = if addr_changes {
                let offset = seg.offset_at(addr_index as u32);
                current_offsets[addr_index] = offset | OFFSET_USE_FLAG;
                offset as usize - 1
            } else {
                let offset = current_offsets[addr_index];
                current_offsets[addr_index] = offset + 1;
                (offset & OFFSET_VALUE_MASK) as usize
            };
            let (row, lane) = lanes.split(islot);
            #[cfg(debug_assertions)]
            {
                assert!(!filled_slots[islot],"RomDataSM: overwriting non empty slot {islot} for mem_op with addr 0x{:X} => 0x{:X} step:{} => {}",
                    rows[row].get_addr(lane) * 8, mem_op.addr * 8, rows[row].get_step(lane), mem_op.step);
                filled_slots[islot] = true;
            }

            rows[row].set_addr(lane, mem_op.addr);
            rows[row].set_step(lane, mem_op.step);

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            rows[row].set_value(lane, 0, low_val);
            rows[row].set_value(lane, 1, high_val);

            rows[row].set_addr_change(lane, addr_changes || (islot == 0 && segment_id == 0));
        }

        let count = mem_ops.len();
        let (last_row, last_lane) = lanes.split(count - 1);
        let last_addr = rows[last_row].get_addr(last_lane);
        let last_value =
            [rows[last_row].get_value(last_lane, 0), rows[last_row].get_value(last_lane, 1)];

        #[cfg(debug_assertions)]
        {
            let mut prev_filled_slot = filled_slots[0];
            let mut from_slot = 0;
            let _count = if is_last_segment { count } else { num_slots };
            for (i, filled) in filled_slots.iter().enumerate().take(_count) {
                debug_assert!(
                    *filled == prev_filled_slot,
                    "RomDataSM: not complete instance found [{}..{}] = {}",
                    from_slot,
                    i - 1,
                    prev_filled_slot
                );
            }
        }

        // The padding lanes repeat the last lane (same addr and value, addr_change = 0), so they
        // are sent to the bus as plain MEMORY_LOAD_OP and never as an INIT: the ROM provides
        // exactly one INIT per address, and that one was already consumed by the first access to
        // this address. The step is pinned to MEMORY_INIT_STEP because that is the step the
        // padding lookup subtracts (`mul: -padding_size` in rom_data.pil), so these extra proves
        // cancel out on the bus.
        //
        // Every padding slot repeats the same values, so the row holding them is built once and
        // the whole rows are overwritten in parallel; only the row the last operation shares with
        // the padding has its lanes set one at a time.
        let padding_size = num_slots - count;
        if padding_size > 0 {
            let lanes_x_row = lanes.lanes();
            let partial_end = count.next_multiple_of(lanes_x_row).min(num_slots);
            for islot in count..partial_end {
                let (row, lane) = lanes.split(islot);
                set_rom_padding_lane::<F, R>(&mut rows[row], lane, last_addr, &last_value);
            }
            let from_row = partial_end / lanes_x_row;
            if from_row < rows.len() {
                let mut pad_row = R::default();
                for lane in 0..lanes_x_row {
                    set_rom_padding_lane::<F, R>(&mut pad_row, lane, last_addr, &last_value);
                }
                rows[from_row..].par_iter_mut().for_each(|row| row.copy_rom_block_from(&pad_row));
            }
        }

        assert!(
            is_last_segment || count == num_slots,
            "All intermediate segments must fill all lanes"
        );

        if is_last_segment {
            self.std.range_check_one(self.range_24bits_id, padding_size as u64);
        }

        RomDataFillOutput { last_addr, last_value, padding_size }
    }

    fn compute_witness_with_offsets_inner<R: RomDataTraceRowOps<F> + RomDataLaneRow<F>>(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = RomDataTrace::<R>::new_from_vec(trace_buffer)?;

        let out = self.fill_trace(&mut trace.buffer, mem_ops, segment_id, is_last_segment, seg);

        let mut air_values = RomDataAirValues::<F>::new();
        Self::set_air_values(&mut air_values, segment_id, is_last_segment, previous_segment, &out);

        #[cfg(feature = "debug_mem")]
        {
            let path = std::env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/rom_trace_{segment_id:04}.txt");
            Self::save_to_file(&trace, &filename);
            Self::dump_trace_to_file(
                &trace,
                &format!("tmp/rom_data_trace_gpu_{segment_id:04}_dump.txt"),
            );
        }

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }

    /// The air values of a `RomData` segment. Shared with the fused `CompactMem` air, whose own
    /// air values carry the same fields under their `rom_` names.
    pub(crate) fn set_air_values(
        air_values: &mut RomDataAirValues<F>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        out: &RomDataFillOutput,
    ) {
        // Force previous_segment_addr = 0 for first instance
        let previous_segment_addr: u32 = if segment_id == 0 { 0 } else { previous_segment.addr };

        air_values.padding_size = F::from_u32(out.padding_size as u32);
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_addr = F::from_u32(previous_segment_addr);
        air_values.segment_last_addr = F::from_u32(out.last_addr);

        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);

        air_values.segment_last_value[0] = F::from_u32(out.last_value[0]);
        air_values.segment_last_value[1] = F::from_u32(out.last_value[1]);
    }

    pub fn dump_trace_to_file<R: RomDataTraceRowOps<F>>(trace: &RomDataTrace<R>, file_name: &str) {
        println!("[RomDataDebug] dumping trace to {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = RomDataTrace::<R>::NUM_ROWS;
        let lanes = lanes_of::<F, R>();

        writeln!(writer, "row lane addr step chunk value").unwrap();
        for i in 0..num_rows {
            for lane in 0..lanes.lanes() {
                let addr = trace[i].get_addr(lane) as u64 * 8;
                let step = trace[i].get_step(lane);
                let chunk = if step == 0 { 0 } else { MemHelpers::mem_step_to_chunk(step).0 };
                let value = trace[i].get_value(lane, 0) as u64
                    | ((trace[i].get_value(lane, 1) as u64) << 32);

                writeln!(writer, "{i} {lane} {addr:#08X} {step} {chunk} 0x{value:X}").unwrap();
            }
        }
        println!("[RomDataDebug] done");
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_to_file<R: RomDataTraceRowOps<F>>(trace: &RomDataTrace<R>, file_name: &str) {
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = RomDataTrace::<R>::NUM_ROWS;
        let lanes = lanes_of::<F, R>();

        for i in 0..num_rows {
            for lane in 0..lanes.lanes() {
                let addr = trace[i].get_addr(lane) * 8;
                let step = trace[i].get_step(lane);
                // TODO: chunk_size * 4 = 20
                writeln!(
                    writer,
                    "{:#010X} {} {:?} @{}",
                    addr,
                    step,
                    trace[i].get_value(lane, 0) as u64
                        + ((trace[i].get_value(lane, 1) as u64) << 32),
                    (step - 1) >> 20
                )
                .unwrap();
            }
        }
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_addr_offsets_to_file<R: RomDataTraceRowOps<F>>(
        trace: &RomDataTrace<R>,
        file_name: &str,
    ) {
        println!("[RomDataDebug] saving address offsets to {} .....", file_name);
        let file = std::fs::File::create(file_name).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        let num_rows = RomDataTrace::<R>::NUM_ROWS;

        let lanes = lanes_of::<F, R>();
        let mut last_addr = u32::MAX;
        // `islot` is the virtual row: the offsets table is expressed in these units.
        for islot in 0..lanes.slots(num_rows) {
            let (row, lane) = lanes.split(islot);
            let addr = trace[row].get_addr(lane);
            if addr != last_addr {
                writeln!(writer, "0x{:08X} {islot}", addr * 8).unwrap();
                last_addr = addr;
            }
        }
        writeln!(writer).unwrap();
        println!("[RomDataDebug] done");
    }
}

/// One padding lane of a `RomData` segment: the last address and value at `MEMORY_INIT_STEP`,
/// with no address change. Kept in one place so the partial row and the whole rows cannot drift
/// apart.
#[inline]
fn set_rom_padding_lane<F: PrimeField64, R: RomDataLaneRow<F>>(
    row: &mut R,
    lane: usize,
    addr: u32,
    value: &[u32; 2],
) {
    row.set_addr(lane, addr);
    row.set_step(lane, MEMORY_INIT_STEP);
    row.set_value(lane, 0, value[0]);
    row.set_value(lane, 1, value[1]);
    row.set_addr_change(lane, false);
}

impl<F: PrimeField64> MemModule<F> for RomDataSM<F> {
    fn get_addr_range(&self) -> (u32, u32) {
        (ROM_DATA_W_ADDR_INIT, ROM_DATA_W_ADDR_END)
    }
    fn is_dual(&self) -> bool {
        false
    }
    fn get_mem_name(&self) -> &str {
        "rom"
    }
    fn is_initializable(&self) -> bool {
        true
    }
    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `MemoryInput` inputs
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(feature = "legacy_mem_count_and_plan", allow(unused_variables))]
    #[inline(always)]
    fn compute_witness(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        #[cfg(not(feature = "legacy_mem_count_and_plan"))]
        {
            self.compute_witness_with_offsets(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                packed,
                seg,
            )
        }
        #[cfg(feature = "legacy_mem_count_and_plan")]
        {
            self.legacy_compute_witness(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                packed,
            )
        }
    }
}
