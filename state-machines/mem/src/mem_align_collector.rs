use crate::MemAlignInput;
use zisk_sm_mem_common::{MemAlignCheckPoint, MemHelpers};

use zisk_common::{
    BusDevice, BusId, ChunkId, CollectCounter, CollectorBytes, MemBusData, MEM_BUS_ID,
};

pub struct MemAlignCollector {
    /// Collected inputs
    pub inputs: Vec<MemAlignInput>,
    pub air_id: usize,
    pub chunk_id: ChunkId,

    full_5: CollectCounter,
    full_3: CollectCounter,
    full_2: CollectCounter,
    read_byte: CollectCounter,
    write_byte: CollectCounter,
}

impl MemAlignCollector {
    /// Inputs a collector reserves for one chunk: every access the plan assigns it there.
    pub fn reserved_inputs(mem_align_checkpoint: &MemAlignCheckPoint) -> usize {
        mem_align_checkpoint.count() as usize
    }

    /// Bytes the collectors of one instance reserve, one collector per chunk checkpoint.
    pub fn capacity_bytes<'a>(
        checkpoints: impl IntoIterator<Item = &'a MemAlignCheckPoint>,
    ) -> CollectorBytes {
        let inputs: usize = checkpoints.into_iter().map(Self::reserved_inputs).sum();
        CollectorBytes::heap((inputs * std::mem::size_of::<MemAlignInput>()) as u64)
    }

    pub fn new(mem_align_checkpoint: &MemAlignCheckPoint) -> Self {
        Self {
            inputs: Vec::with_capacity(Self::reserved_inputs(mem_align_checkpoint)),
            air_id: mem_align_checkpoint.air_id,
            chunk_id: mem_align_checkpoint.chunk_id,
            full_5: mem_align_checkpoint.full_5,
            full_3: mem_align_checkpoint.full_3,
            full_2: mem_align_checkpoint.full_2,
            read_byte: mem_align_checkpoint.read_byte,
            write_byte: mem_align_checkpoint.write_byte,
        }
    }
    fn input_push_read(&mut self, addr: u64, bytes: u8, data: &[u64]) {
        let mem_values = MemBusData::get_mem_values(data);
        self.inputs.push(MemAlignInput {
            addr,
            is_write: false,
            width: bytes,
            value: MemHelpers::get_read_value(addr, bytes, mem_values),
            step: MemBusData::get_step(data),
            mem_values,
        });
    }
    fn input_push_write(&mut self, addr: u64, bytes: u8, data: &[u64]) {
        self.inputs.push(MemAlignInput {
            addr,
            is_write: true,
            width: bytes,
            value: MemBusData::get_value(data),
            step: MemBusData::get_step(data),
            mem_values: MemBusData::get_mem_values(data),
        });
    }
    pub fn count(&self) -> u32 {
        self.full_2.count()
            + self.full_3.count()
            + self.full_5.count()
            + self.read_byte.count()
            + self.write_byte.count()
    }

    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        debug_assert!(*bus_id == MEM_BUS_ID);

        let bytes = MemBusData::get_bytes(data);

        if bytes == 1 {
            let is_write = MemHelpers::is_write(MemBusData::get_op(data));
            if is_write {
                if (MemBusData::get_value(data) & 0xFFFF_FFFF_FFFF_FF00) == 0 {
                    if !self.write_byte.should_skip() {
                        self.input_push_write(MemBusData::get_addr(data), bytes, data);
                    }
                    return true;
                }
            } else {
                if !self.read_byte.should_skip() {
                    self.input_push_read(MemBusData::get_addr(data), bytes, data);
                }
                return true;
            }
        }

        let addr = MemBusData::get_addr(data);
        if MemHelpers::is_aligned(addr, bytes) {
            return true;
        }

        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        let ops_by_addr = if MemHelpers::is_double(addr, bytes) { 2 } else { 1 };
        let rows = if is_write { 1 + 2 * ops_by_addr } else { 1 + ops_by_addr };
        match rows as u8 {
            5 => {
                if !self.full_5.should_skip() {
                    self.input_push_write(addr, bytes, data);
                }
            }
            3 => {
                if !self.full_3.should_skip() {
                    if is_write {
                        self.input_push_write(addr, bytes, data);
                    } else {
                        self.input_push_read(addr, bytes, data);
                    }
                }
            }
            2 => {
                if !self.full_2.should_skip() {
                    self.input_push_read(addr, bytes, data);
                }
            }
            _ => panic!("Invalid mem_align_op_rows {rows}"),
        };
        true
    }
}

impl BusDevice<u64> for MemAlignCollector {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_point(counts: [u32; 5]) -> MemAlignCheckPoint {
        let counter = |count| CollectCounter::new(0, count);
        MemAlignCheckPoint {
            air_id: 0,
            chunk_id: ChunkId(0),
            full_2: counter(counts[0]),
            full_3: counter(counts[1]),
            full_5: counter(counts[2]),
            read_byte: counter(counts[3]),
            write_byte: counter(counts[4]),
        }
    }

    /// The instances bound their collectors' bytes with `reserved_inputs`, so `new` must reserve
    /// exactly that.
    #[test]
    fn new_reserves_the_inputs_the_bound_counts() {
        let check_point = check_point([1, 2, 3, 4, 5]);
        assert_eq!(MemAlignCollector::reserved_inputs(&check_point), 15);
        assert_eq!(
            MemAlignCollector::new(&check_point).inputs.capacity(),
            MemAlignCollector::reserved_inputs(&check_point)
        );
    }

    #[test]
    fn capacity_bytes_sums_every_chunk() {
        let first = check_point([10, 0, 0, 0, 0]);
        let second = check_point([0, 0, 0, 2, 3]);
        assert_eq!(
            MemAlignCollector::capacity_bytes([&first, &second]).total(),
            15 * std::mem::size_of::<MemAlignInput>() as u64
        );
        assert_eq!(
            MemAlignCollector::capacity_bytes(std::iter::empty()),
            CollectorBytes::default()
        );
    }
}
