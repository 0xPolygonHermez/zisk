//! Collects the `jump_dest` operations one chunk contributes to a segment.
//!
//! The plan is expressed in rows, and an operation spans many of them, so a
//! segment can begin or end inside one. The collector therefore keeps a running
//! row cursor over the chunk and keeps every operation whose row range overlaps
//! the window `[skip, skip + rows)` this segment was given — including the one
//! that started in a previous segment. `first_row_offset` records how far into
//! that first operation the window begins, which is what lets the instance drop
//! the rows that are not its own.

use std::any::Any;

use zisk_common::{
    BusDevice, BusDeviceMode, BusId, ChunkId, CollectSkipper, PayloadType, A, B, OP,
    OPERATION_BUS_ID, OPERATION_PRECOMPILED_BUS_DATA_SIZE, STEP,
};
use zisk_core::zisk_ops::ZiskOp;

use crate::jump_dest_rows;

/// One `jump_dest` operation, with the payload needed to rebuild its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JumpDestInput {
    /// Bitmap address, from the `a` operand.
    pub bitmap_addr: u64,
    /// Bytecode address, from the `b` operand.
    pub bytecode_addr: u64,
    /// Main step, the timestamp the memory operations carry.
    pub main_step: u64,
    /// Bytecode bytes.
    pub count: u64,
    /// Every source word the range spans, straight from the minimal trace.
    pub words: Vec<u64>,
}

impl JumpDestInput {
    /// Rows this operation occupies in the AIR.
    #[inline]
    pub fn rows(&self) -> u64 {
        jump_dest_rows(self.count as usize) as u64
    }
}

#[derive(Debug)]
pub struct JumpDestCollector {
    /// Operations overlapping this segment's window, in bus order.
    pub inputs: Vec<JumpDestInput>,

    /// Rows this segment takes from the chunk.
    pub rows: u64,

    /// Rows of the first collected operation that belong to an earlier segment.
    pub first_row_offset: u64,

    /// Rows already accounted for, of the ones this segment wants.
    collected_rows: u64,

    /// Walks past the rows consumed by earlier segments.
    skipper: CollectSkipper,

    /// Rows seen so far in this chunk, including the skipped ones.
    cursor: u64,

    pub chunk_id: ChunkId,

    /// True when this chunk closes the segment.
    pub last_chunk: bool,
}

impl JumpDestCollector {
    pub fn new(chunk_id: ChunkId, rows: u64, skipper: CollectSkipper, last_chunk: bool) -> Self {
        Self {
            inputs: Vec::new(),
            rows,
            first_row_offset: 0,
            collected_rows: 0,
            skipper,
            cursor: 0,
            chunk_id,
            last_chunk,
        }
    }

    /// Takes the collected operations, leaving the collector empty.
    pub fn take_inputs(mut self) -> Vec<JumpDestInput> {
        std::mem::take(&mut self.inputs)
    }

    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64], data_ext: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP] as u8 != ZiskOp::JUMP_DEST {
            return true;
        }
        if self.collected_rows >= self.rows {
            return true;
        }

        let count = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE];
        let op_rows = jump_dest_rows(count as usize) as u64;
        let skip = self.skipper.skip;

        // Rows of this operation, as an interval of the chunk's row stream.
        let start = self.cursor;
        let end = start + op_rows;
        self.cursor = end;

        // Entirely before the window: nothing of it belongs here.
        if end <= skip {
            return true;
        }

        if self.inputs.is_empty() {
            self.first_row_offset = skip.saturating_sub(start);
        }

        self.inputs.push(JumpDestInput {
            bitmap_addr: data[A],
            bytecode_addr: data[B],
            main_step: data[STEP],
            count,
            words: data_ext.to_vec(),
        });
        self.collected_rows += end.min(skip + self.rows) - start.max(skip);

        true
    }
}

impl BusDevice<PayloadType> for JumpDestCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// Mode the collector runs in, kept for symmetry with the counter side.
pub const JUMP_DEST_COLLECTOR_MODE: BusDeviceMode = BusDeviceMode::InputGenerator;

#[cfg(test)]
mod tests {
    use zisk_core::ZiskOperationType;

    use super::*;
    use crate::JUMP_DEST_ROWS_X_BLOCK;

    fn bus_data(count: u64) -> Vec<u64> {
        vec![0xc0, ZiskOperationType::Evm as u64, 0xA000_0000, 0xA001_0000, 7, count]
    }

    /// Feeds `ops` operations of `count` bytes each into a collector with the
    /// given window, and reports what it kept.
    fn collect(counts: &[u64], rows: u64, skip: u64) -> (Vec<u64>, u64, u64) {
        let mut collector =
            JumpDestCollector::new(ChunkId(0), rows, CollectSkipper::new(skip), true);
        for &count in counts {
            let words = vec![0u64; count.div_ceil(8) as usize];
            collector.process_data(&OPERATION_BUS_ID, &bus_data(count), &words);
        }
        (
            collector.inputs.iter().map(|i| i.count).collect(),
            collector.first_row_offset,
            collector.collected_rows,
        )
    }

    #[test]
    fn a_window_covering_everything_keeps_every_operation() {
        let (kept, offset, rows) = collect(&[64, 64, 64], 3 * JUMP_DEST_ROWS_X_BLOCK as u64, 0);
        assert_eq!(kept, vec![64, 64, 64]);
        assert_eq!(offset, 0);
        assert_eq!(rows, 3 * JUMP_DEST_ROWS_X_BLOCK as u64);
    }

    #[test]
    fn operations_before_the_window_are_dropped() {
        // Each 64-byte operation is one block. Skipping two blocks drops the
        // first two operations outright.
        let block = JUMP_DEST_ROWS_X_BLOCK as u64;
        let (kept, offset, _) = collect(&[64, 64, 64], block, 2 * block);
        assert_eq!(kept, vec![64]);
        assert_eq!(offset, 0, "the window starts exactly on an operation boundary");
    }

    #[test]
    fn a_window_starting_mid_operation_records_the_offset() {
        // One operation of 192 bytes spans three blocks; the window starts one
        // block into it.
        let block = JUMP_DEST_ROWS_X_BLOCK as u64;
        let (kept, offset, rows) = collect(&[192], 2 * block, block);
        assert_eq!(kept, vec![192], "the straddling operation is still needed");
        assert_eq!(offset, block, "its first block belongs to the previous segment");
        assert_eq!(rows, 2 * block);
    }

    #[test]
    fn collecting_stops_once_the_window_is_full() {
        let block = JUMP_DEST_ROWS_X_BLOCK as u64;
        let (kept, _, rows) = collect(&[64, 64, 64, 64], 2 * block, 0);
        assert_eq!(kept, vec![64, 64], "the rest belongs to the next segment");
        assert_eq!(rows, 2 * block);
    }
}
