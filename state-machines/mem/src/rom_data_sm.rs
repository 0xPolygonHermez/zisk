use std::sync::Arc;

use crate::{mem_sm::MemPreviousSegment, MemModule, MemOps};
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

use crate::mem_gpu_fill::{gpu_fill_mode, GpuFillMode};

/// The scalars a RomData instance's air values are built from: the last lane and the padding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RomDataFillOutput {
    last_addr: u32,
    last_value: [u32; 2],
    padding_size: u32,
}

impl RomDataFillOutput {
    fn scalars(
        &self,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
    ) -> [u64; 8] {
        [
            usize::from(segment_id) as u64,
            is_last_segment as u64,
            previous_segment.addr as u64,
            previous_segment.value,
            self.last_addr as u64,
            self.last_value[0] as u64,
            self.last_value[1] as u64,
            self.padding_size as u64,
        ]
    }

    fn air_values<F: PrimeField64>(
        &self,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
    ) -> RomDataAirValues<'static, F> {
        let mut air_values = RomDataAirValues::<F>::new();
        air_values.padding_size = F::from_u32(self.padding_size);
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_addr =
            F::from_u32(if segment_id == 0 { 0 } else { previous_segment.addr });
        air_values.segment_last_addr = F::from_u32(self.last_addr);
        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);
        air_values.segment_last_value[0] = F::from_u32(self.last_value[0]);
        air_values.segment_last_value[1] = F::from_u32(self.last_value[1]);
        air_values
    }
}
use zisk_sm_mem_common::{
    MemHelpers, MemLanes, MemModuleSegmentCheckPoint, MEMORY_INIT_STEP, MEM_BYTES_BITS,
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

/// One padding lane: the last lane repeated with `addr_change` cleared. Kept in one place so the
/// partial row and the whole rows cannot drift apart.
///
/// Every column of the row is written here, which is what lets the whole rows be filled by copying
/// one built row rather than by setting each column of each lane.
#[inline]
fn set_rom_data_padding_lane<F: PrimeField64, R: RomDataTraceRowOps<F>>(
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

pub struct RomDataSM<F: PrimeField64> {
    _phantom: std::marker::PhantomData<F>,
}

const OFFSET_USE_FLAG: u32 = 0x8000_0000;
const OFFSET_VALUE_MASK: u32 = 0x7FFF_FFFF;

#[allow(unused, unused_variables)]
impl<F: PrimeField64> RomDataSM<F> {
    /// Takes no `Std`: the only thing this machine used it for was range checks, which the prover
    /// now computes from the committed trace.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { _phantom: std::marker::PhantomData })
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
            // `ZISK_MEM_GPU_FILL=arena-check`: the device rows against the CPU rows, every word
            // and scalar; the CPU rows are proved.
            let seg_idx = usize::from(segment_id);
            let check = gpu_fill_mode() == GpuFillMode::ArenaCheck;
            let used_bits = Self::packed_used_bits();
            let mut hook = move |cpu: &[u64], out: &RomDataFillOutput| {
                let n_rows = RomDataTrace::<RomDataTraceRowPacked<F>>::NUM_ROWS;
                let words_per_row = RomDataTraceRowPacked::<F>::PACKED_WORDS;
                let mut gpu = vec![0u64; n_rows * words_per_row];
                match Self::device_rows(&mut gpu, n_rows, segment_id) {
                    Ok((report, prev)) => {
                        let (count, first) = crate::mem_gpu_fill::compare_rows_masked(
                            cpu,
                            &gpu,
                            words_per_row,
                            used_bits,
                        );
                        let scalars_ok = report == *out
                            && prev.addr == previous_segment.addr
                            && prev.value == previous_segment.value;
                        tracing::info!(
                            "RomData[{seg_idx}] arena CHECK: {} words differ{} | scalars {}{}",
                            count,
                            first
                                .map(|(row, w, c, g)| format!(
                                    ", first row {row} word {w}: cpu {c:#x} gpu {g:#x}"
                                ))
                                .unwrap_or_default(),
                            if scalars_ok { "match" } else { "DIFFER" },
                            if scalars_ok {
                                String::new()
                            } else {
                                format!(
                                    " (cpu prev {:#x}/{:#x} {:?}; gpu prev {:#x}/{:#x} {:?})",
                                    previous_segment.addr,
                                    previous_segment.value,
                                    out,
                                    prev.addr,
                                    prev.value,
                                    report
                                )
                            }
                        );
                    }
                    Err(e) => tracing::warn!("RomData[{seg_idx}] arena CHECK unavailable: {e}"),
                }
            };
            self.compute_witness_with_offsets_inner::<RomDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
                if check { Some(&mut hook) } else { None },
                used_bits,
            )
        } else {
            self.compute_witness_with_offsets_inner::<RomDataTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
                None,
                std::mem::size_of::<RomDataTraceRow<F>>() * 8,
            )
        }
    }

    /// The bits a packed RomData row uses.
    fn packed_used_bits() -> usize {
        crate::mem_gpu_fill::packed_used_bits(
            RomDataTrace::<()>::AIRGROUP_ID,
            RomDataTrace::<()>::AIR_ID,
        )
        .unwrap_or(RomDataTraceRowPacked::<F>::PACKED_WORDS * 64)
    }

    /// The rows of RomData instance `segment_id` from the GPU planner, with the fill's scalars and
    /// the lane before the instance.
    fn device_rows(
        rows: &mut [u64],
        n_rows: usize,
        segment_id: SegmentId,
    ) -> Result<(RomDataFillOutput, MemPreviousSegment), String> {
        let res = zisk_sm_mem_planner::gpu_rom_witness_fill(
            usize::from(segment_id) as u32,
            rows,
            n_rows as u32,
        )?;
        let num_slots = n_rows * zisk_sm_mem_common::rom_data_lanes_x_row();
        let out = RomDataFillOutput {
            last_addr: res.last_addr_w,
            last_value: [res.last_value as u32, (res.last_value >> 32) as u32],
            padding_size: (num_slots - res.n_lanes as usize) as u32,
        };
        let prev = MemPreviousSegment {
            addr: res.prev_addr_w,
            step: res.prev_step,
            value: res.prev_value,
        };
        Ok((out, prev))
    }
    /// Fills the witness trace using a precomputed **offset table** (GPU path).
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
    ///
    /// `on_filled` sees the filled rows (as words) and the fill's scalars before the air instance
    /// is built: the arena check compares them with the device rows.
    #[allow(clippy::too_many_arguments)]
    fn compute_witness_with_offsets_inner<R: RomDataTraceRowOps<F>>(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        seg: &MemModuleSegmentCheckPoint,
        on_filled: crate::mem_gpu_fill::OnFilled<'_, RomDataFillOutput>,
        used_bits: usize,
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
        // `current_offsets` packs a 1-based virtual row plus a flag bit in a u32.
        debug_assert!(
            num_slots < OFFSET_USE_FLAG as usize,
            "RomDataSM: {num_slots} virtual rows do not fit in OFFSET_VALUE_MASK"
        );
        // save_offsets_to_file(
        //     seg,
        //     &format!("tmp/rom_data_trace_gpu_{segment_id:04}_offsets.txt"),
        // );
        let previous_segment_addr: u32 = if segment_id == 0 { 0 } else { previous_segment.addr };
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
                    trace[row].get_addr(lane) * 8, mem_op.addr * 8, trace[row].get_step(lane), mem_op.step);
                filled_slots[islot] = true;
            }

            trace[row].set_addr(lane, mem_op.addr);
            trace[row].set_step(lane, mem_op.step);

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[row].set_value(lane, 0, low_val);
            trace[row].set_value(lane, 1, high_val);

            trace[row].set_addr_change(lane, addr_changes || (islot == 0 && segment_id == 0));
        }

        let count = mem_ops.len();
        let (last_row, last_lane) = lanes.split(count - 1);
        let last_addr = trace[last_row].get_addr(last_lane);
        let last_value =
            [trace[last_row].get_value(last_lane, 0), trace[last_row].get_value(last_lane, 1)];

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
        // Every padding lane holds the same values, so only the row the last operation shares with
        // the padding is written lane by lane; the whole rows after it are one built row copied
        // over them in parallel, which is a row-sized move instead of a setter per column.
        if count < num_slots {
            let lanes_x_row = lanes.lanes();
            let partial_end = count.next_multiple_of(lanes_x_row).min(num_slots);
            for islot in count..partial_end {
                let (row, lane) = lanes.split(islot);
                set_rom_data_padding_lane::<F, R>(&mut trace[row], lane, last_addr, &last_value);
            }
            let from_row = partial_end / lanes_x_row;
            if from_row < RomDataTrace::<R>::NUM_ROWS {
                let mut pad_row = R::default();
                for lane in 0..lanes_x_row {
                    set_rom_data_padding_lane::<F, R>(&mut pad_row, lane, last_addr, &last_value);
                }
                trace.buffer[from_row..].par_iter_mut().for_each(|row| *row = pad_row);
            }
        }

        assert!(
            is_last_segment || count == num_slots,
            "All intermediate segments must fill all lanes"
        );

        let out =
            RomDataFillOutput { last_addr, last_value, padding_size: (num_slots - count) as u32 };
        {
            let words = crate::mem_trace_hash::rows_as_words(&trace.buffer);
            if let Some(hook) = on_filled {
                hook(words, &out);
            }
            crate::mem_trace_hash::dump_scalars(
                self.get_mem_name(),
                usize::from(segment_id),
                words,
                &out.scalars(segment_id, is_last_segment, previous_segment),
                std::mem::size_of::<R>() / 8,
                used_bits,
            );
        }
        let mut air_values = out.air_values::<F>(segment_id, is_last_segment, previous_segment);
        debug_assert_eq!(air_values.previous_segment_addr, F::from_u32(previous_segment_addr));

        #[cfg(feature = "debug_mem")]
        {
            let path = std::env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/rom_trace_{segment_id:04}.txt");
            Self::save_to_file(&trace, &filename);
        }

        #[cfg(feature = "debug_mem")]
        Self::dump_trace_to_file(
            &trace,
            &format!("tmp/rom_data_trace_gpu_{segment_id:04}_dump.txt"),
        );
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
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

impl<F: PrimeField64> MemModule<F> for RomDataSM<F> {
    fn compute_witness_gpu_arena(
        &self,
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        if !packed {
            return Err(proofman_common::ProofmanError::InvalidParameters(
                "ZISK_MEM_GPU_FILL=arena needs the packed RomData trace".to_string(),
            ));
        }
        let seg_idx = usize::from(segment_id);
        // The device rows cover every row of the instance, padding included.
        let mut trace = RomDataTrace::<RomDataTraceRowPacked<F>>::new_from_vec(trace_buffer)?;
        let n_rows = trace.num_rows();
        let (out, previous_segment) = {
            let words = crate::mem_trace_hash::rows_as_words_mut(&mut trace.buffer);
            Self::device_rows(words, n_rows, segment_id)
        }
        .map_err(|e| {
            proofman_common::ProofmanError::InvalidParameters(format!(
                "RomData[{seg_idx}] witness from the GPU planner failed: {e}"
            ))
        })?;
        assert!(
            is_last_segment || out.padding_size == 0,
            "RomDataSM: padding_size must be 0 for non last segment, but got {}",
            out.padding_size
        );
        crate::mem_trace_hash::dump_scalars(
            self.get_mem_name(),
            seg_idx,
            crate::mem_trace_hash::rows_as_words(&trace.buffer),
            &out.scalars(segment_id, is_last_segment, &previous_segment),
            RomDataTraceRowPacked::<F>::PACKED_WORDS,
            Self::packed_used_bits(),
        );
        let mut air_values = out.air_values::<F>(segment_id, is_last_segment, &previous_segment);
        Ok(Some(AirInstance::new_from_trace(
            FromTrace::new(&mut trace).with_air_values(&mut air_values),
        )))
    }

    fn compute_witness_gpu_slot(
        &self,
        decl: &proofman_common::GpuWitnessAir,
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        use proofman_common::trace::TraceRow;
        if !packed {
            return Err(proofman_common::ProofmanError::InvalidParameters(
                "ZISK_MEM_GPU_FILL=slot needs the packed RomData trace".to_string(),
            ));
        }
        let seg_idx = usize::from(segment_id);
        let n_rows = RomDataTrace::<RomDataTraceRowPacked<F>>::NUM_ROWS;
        let res = zisk_sm_mem_planner::gpu_mem_witness_scalars(
            crate::mem_gpu_fill::SLOT_FAMILY_ROM,
            seg_idx as u32,
        )
        .map_err(|e| {
            proofman_common::ProofmanError::InvalidParameters(format!(
                "RomData[{seg_idx}] scalars from the GPU planner failed: {e}"
            ))
        })?;
        let num_slots = n_rows * zisk_sm_mem_common::rom_data_lanes_x_row();
        let out = RomDataFillOutput {
            last_addr: res.last_addr_w,
            last_value: [res.last_value as u32, (res.last_value >> 32) as u32],
            padding_size: (num_slots - res.n_lanes as usize) as u32,
        };
        let previous_segment = MemPreviousSegment {
            addr: res.prev_addr_w,
            step: res.prev_step,
            value: res.prev_value,
        };
        let mut air_values = out.air_values::<F>(segment_id, is_last_segment, &previous_segment);
        crate::mem_gpu_fill::slot_instance(
            decl,
            crate::mem_gpu_fill::SLOT_FAMILY_ROM,
            RomDataTrace::<()>::AIR_ID,
            seg_idx,
            n_rows,
            RomDataTraceRow::<F>::ROW_SIZE,
            trace_buffer,
            proofman_common::trace::Values::get_buffer(&mut air_values),
        )
        .map(Some)
    }

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
