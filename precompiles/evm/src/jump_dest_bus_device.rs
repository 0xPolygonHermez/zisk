//! Counter and mem-input generator for the `jump_dest` precompile.
//!
//! Two jobs on the operation bus:
//!
//! * **Counter** — how many AIR rows the operations seen so far need, which is
//!   what the planner segments.
//! * **Input generator** — the memory operations the precompile performs, so the
//!   memory state machine accounts for them. These are *not* uniform: only the
//!   source words the walk actually loads are read, and which ones those are
//!   depends on the bytecode, so they have to be derived by walking it.
//!
//! The operation payload is `[op, op_type, a, b, step]` followed by the
//! minimal-trace header (the byte count); `data_ext` carries the payload proper,
//! every source word the byte range spans.

use std::fmt;

use precompiles_common::{MemBusHelpers, MemProcessor};
use std::ops::Add;

use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Metrics, A, B, OP, OPERATION_BUS_ID,
    OPERATION_PRECOMPILED_BUS_DATA_SIZE, OP_TYPE, STEP,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType, EXTRA_PARAMS_ADDR};

use precompiles_helpers::{bitmap_words, src_words, walk_jump_dest_bitmap, BYTES_PER_WORD};

/// Operations per row in the `JumpDest` AIR — mirrors `op_x_row` in
/// `pil/jump_dest.pil`.
pub const JUMP_DEST_OPS_X_ROW: usize = 2;

/// Rows per block of 8 ops, i.e. per 64-byte chunk of bytecode.
pub const JUMP_DEST_ROWS_X_BLOCK: usize = 8 / JUMP_DEST_OPS_X_ROW;

/// The header word of the minimal trace sits right after the fixed operation
/// fields; the source words follow in `data_ext`.
const COUNT_OFFSET: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE;

/// Rows the AIR needs for one operation of `count` bytes. The machine always
/// writes whole bitmap words, so the row count follows from the block count and
/// never from where the bytecode happens to end.
#[inline]
pub fn jump_dest_rows(count: usize) -> usize {
    bitmap_words(count) * JUMP_DEST_ROWS_X_BLOCK
}

/// Counts rows and derives the memory operations of `jump_dest`.
#[derive(Debug)]
pub struct JumpDestCounterInputGen {
    /// AIR rows needed by every operation seen so far.
    pub rows: usize,
    mode: BusDeviceMode,
}

impl fmt::Display for JumpDestCounterInputGen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JumpDest rows: {}", self.rows)
    }
}

impl JumpDestCounterInputGen {
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { rows: 0, mode }
    }

    /// Emits the memory operations of one `jump_dest`: the count read, one read
    /// per source word the walk loads, and one write per bitmap word.
    ///
    /// `only_counters` spares the bitmap computation when the values are not
    /// needed, but the walk still has to run — the *set* of reads depends on the
    /// bytecode, so it cannot be shortcut the way a uniform precompile can.
    pub fn generate_mem_inputs<P: MemProcessor>(
        data: &[u64],
        data_ext: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        let bitmap_addr = data[A] as u32;
        let bytecode_addr = data[B] as u32;
        let step = data[STEP];
        let count = data[COUNT_OFFSET] as usize;

        // The opcode reads its byte count from the extra-parameter slot.
        MemBusHelpers::mem_aligned_op(
            EXTRA_PARAMS_ADDR as u32,
            step,
            count as u64,
            false,
            mem_processors,
        );

        if count == 0 {
            return;
        }
        debug_assert_eq!(data_ext.len(), src_words(count), "payload must span the whole range");

        let mut bitmap = vec![0u64; bitmap_words(count)];
        walk_jump_dest_bitmap(
            count,
            |word_index| {
                let word = data_ext[word_index];
                // Only the words the walk visits are read from memory.
                MemBusHelpers::mem_aligned_op(
                    bytecode_addr + (word_index * BYTES_PER_WORD) as u32,
                    step,
                    word,
                    false,
                    mem_processors,
                );
                word
            },
            &mut bitmap,
        );

        // Every bitmap word is written, zeros included.
        for (index, word) in bitmap.iter().enumerate() {
            let value = if only_counters { 0 } else { *word };
            MemBusHelpers::mem_aligned_op(
                bitmap_addr + (index * BYTES_PER_WORD) as u32,
                step,
                value,
                true,
                mem_processors,
            );
        }
    }

    /// In `InputGenerator` mode an operation may be skipped when no collector
    /// cares about **any** address it touches. All three ranges have to be
    /// checked: the count read, the bitmap it writes, and the bytecode it reads.
    /// Leaving the bytecode out would skip an operation whose reads a collector
    /// is waiting for, and the memory bus would not balance.
    fn should_skip<P: MemProcessor>(data: &[u64], mem_processors: &mut P) -> bool {
        let count = data[COUNT_OFFSET] as usize;
        if count == 0 {
            // Unreachable: count > 0 is a precondition the emulator asserts. Kept total anyway,
            // and an empty call would touch nothing but the count read.
            return mem_processors.skip_addr(EXTRA_PARAMS_ADDR as u32);
        }
        let bitmap_addr = data[A] as u32;
        let bitmap_last = bitmap_addr + ((bitmap_words(count) - 1) * BYTES_PER_WORD) as u32;
        let bytecode_addr = data[B] as u32;
        let bytecode_last = bytecode_addr + ((src_words(count) - 1) * BYTES_PER_WORD) as u32;

        mem_processors.skip_addr(EXTRA_PARAMS_ADDR as u32)
            && mem_processors.skip_addr_range(bitmap_addr, bitmap_last)
            && mem_processors.skip_addr_range(bytecode_addr, bytecode_last)
    }

    #[inline(always)]
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        data_ext: &[u64],
        mem_processors: &mut P,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP] as u8 != ZiskOp::JUMP_DEST {
            return true;
        }

        match self.mode {
            BusDeviceMode::Counter => {
                Metrics::measure(self, data);
                Self::generate_mem_inputs(data, data_ext, true, mem_processors);
            }
            // Under the ASM emulator the memory operations already come from the
            // mops trace, so only the row count is needed here.
            BusDeviceMode::CounterAsm => Metrics::measure(self, data),
            BusDeviceMode::InputGenerator => {
                if Self::should_skip(data, mem_processors) {
                    return true;
                }
                Self::generate_mem_inputs(data, data_ext, false, mem_processors);
            }
        }
        true
    }
}

impl Metrics for JumpDestCounterInputGen {
    /// Adds the rows one operation needs. Only the byte count matters: the
    /// machine always writes whole bitmap words.
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        if data[OP_TYPE] != ZiskOperationType::Evm as u64 {
            return;
        }
        self.rows += jump_dest_rows(data[COUNT_OFFSET] as usize);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for JumpDestCounterInputGen {
    type Output = JumpDestCounterInputGen;

    fn add(self, other: Self) -> JumpDestCounterInputGen {
        JumpDestCounterInputGen { rows: self.rows + other.rows, mode: self.mode.clone() }
    }
}

impl Add<&JumpDestCounterInputGen> for &JumpDestCounterInputGen {
    type Output = JumpDestCounterInputGen;

    fn add(self, other: &JumpDestCounterInputGen) -> JumpDestCounterInputGen {
        JumpDestCounterInputGen { rows: self.rows + other.rows, mode: self.mode.clone() }
    }
}

impl BusDevice<u64> for JumpDestCounterInputGen {
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records the memory operations a generation emits.
    #[derive(Default)]
    struct Recorder {
        ops: Vec<(u32, bool)>,
    }
    impl MemProcessor for Recorder {
        fn process_mem_data(&mut self, data: &[u64; 7]) {
            // [op, addr, step, bytes, value_lo, value_hi, ...] — op 2 is a store.
            self.ops.push((data[1] as u32, data[0] == 2));
        }
        fn skip_addr(&mut self, _addr: u32) -> bool {
            false
        }
        fn skip_addr_range(&mut self, _from: u32, _to: u32) -> bool {
            false
        }
    }

    fn payload(bytecode: &[u8]) -> Vec<u64> {
        (0..src_words(bytecode.len()))
            .map(|w| {
                let mut bytes = [0u8; 8];
                let offset = w * 8;
                let available = std::cmp::min(8, bytecode.len() - offset);
                bytes[..available].copy_from_slice(&bytecode[offset..offset + available]);
                u64::from_le_bytes(bytes)
            })
            .collect()
    }

    fn bus_data(count: usize) -> Vec<u64> {
        // [op, op_type, a = bitmap, b = bytecode, step, count]
        vec![0xc0, ZiskOperationType::Evm as u64, 0xA000_0000, 0xA001_0000, 7, count as u64]
    }

    #[test]
    fn rows_follow_the_block_count_not_the_byte_count() {
        assert_eq!(jump_dest_rows(0), 0);
        assert_eq!(jump_dest_rows(1), JUMP_DEST_ROWS_X_BLOCK);
        assert_eq!(jump_dest_rows(64), JUMP_DEST_ROWS_X_BLOCK);
        assert_eq!(jump_dest_rows(65), 2 * JUMP_DEST_ROWS_X_BLOCK);
    }

    #[test]
    fn only_the_loaded_words_are_read() {
        // PUSH32 every 33 bytes: words 0, 4, 8, 12 and the trailing partial one
        // are the only ones the walk visits.
        let mut bytecode = vec![0x00u8; 132];
        for pc in (0..132).step_by(33) {
            bytecode[pc] = 0x7f;
        }
        let mut recorder = Recorder::default();
        JumpDestCounterInputGen::generate_mem_inputs(
            &bus_data(bytecode.len()),
            &payload(&bytecode),
            false,
            &mut recorder,
        );

        let reads: Vec<u32> = recorder
            .ops
            .iter()
            .filter(|(addr, is_write)| !is_write && *addr != EXTRA_PARAMS_ADDR as u32)
            .map(|(addr, _)| (addr - 0xA001_0000) / 8)
            .collect();
        assert_eq!(reads, vec![0, 4, 8, 12, 16]);

        let writes: Vec<u32> = recorder
            .ops
            .iter()
            .filter(|(_, is_write)| *is_write)
            .map(|(addr, _)| (addr - 0xA000_0000) / 8)
            .collect();
        assert_eq!(writes, vec![0, 1, 2], "every bitmap word is written, zeros included");
    }

    #[test]
    fn dense_code_reads_every_word() {
        let bytecode = vec![0x5bu8; 200];
        let mut recorder = Recorder::default();
        JumpDestCounterInputGen::generate_mem_inputs(
            &bus_data(bytecode.len()),
            &payload(&bytecode),
            false,
            &mut recorder,
        );
        let reads = recorder
            .ops
            .iter()
            .filter(|(addr, is_write)| !is_write && *addr != EXTRA_PARAMS_ADDR as u32)
            .count();
        assert_eq!(reads, src_words(200));
    }

    #[test]
    fn an_empty_operation_only_reads_its_count() {
        let mut recorder = Recorder::default();
        JumpDestCounterInputGen::generate_mem_inputs(&bus_data(0), &[], false, &mut recorder);
        assert_eq!(recorder.ops, vec![(EXTRA_PARAMS_ADDR as u32, false)]);
    }
}
