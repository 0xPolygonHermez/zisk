use std::collections::HashMap;

use sm_common::Metrics;
use zisk_common::{BusDevice, BusId};
use zisk_core::ZiskOperationType;

use crate::{
    MEMORY_MAX_DIFF, MEMORY_STORE_OP, MEM_BUS_ID, MEM_BYTES_BITS, MEM_REGS_ADDR, MEM_REGS_MASK,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct UsesCounter {
    pub first_step: u64,
    pub last_step: u64,
    pub count: u64,
}

pub struct MemCounters {
    registers: [UsesCounter; 32],
    addr: HashMap<u32, UsesCounter>,
    mem_align: Vec<u8>,
    mem_align_rows: u32,
}

impl MemCounters {
    pub fn new() -> Self {
        let empty_counter = UsesCounter::default();
        Self {
            registers: [empty_counter; 32],
            addr: HashMap::new(),
            mem_align: Vec::new(),
            mem_align_rows: 0,
        }
    }
    pub fn count_extra_internal_reads(previous_step: u64, step: u64) -> u64 {
        let diff = step - previous_step;
        if diff > MEMORY_MAX_DIFF {
            (diff - 1) / MEMORY_MAX_DIFF
        } else {
            0
        }
    }
}

impl Metrics for MemCounters {
    fn measure(&mut self, _: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        let op = data[0] as u8;
        let addr = data[1] as u32;
        let mut addr_w = addr >> MEM_BYTES_BITS;
        let step = data[2];
        let bytes = data[3] as u8;

        if (addr & MEM_REGS_MASK) == MEM_REGS_ADDR {
            let reg_index = ((addr >> 3) & 0x1F) as usize;
            if self.registers[reg_index].count == 0 {
                self.registers[reg_index] =
                    UsesCounter { first_step: step, last_step: step, count: 1 };
            } else {
                self.registers[reg_index].count +=
                    1 + Self::count_extra_internal_reads(self.registers[reg_index].last_step, step);
                self.registers[reg_index].last_step = step;
            }
        } else {
            let aligned = addr & 0x7 == 0 && bytes == 8;
            if aligned {
                self.addr
                    .entry(addr_w)
                    .and_modify(|value| {
                        value.count += 1;
                        value.last_step = step;
                    })
                    .or_insert(UsesCounter { first_step: step, last_step: step, count: 1 });
            } else {
                // TODO: use mem_align helpers

                let addr_count =
                    if ((addr + bytes as u32) >> MEM_BYTES_BITS) != addr_w { 2 } else { 1 };
                let ops_by_addr = if op == MEMORY_STORE_OP { 2 } else { 1 };

                let last_step = step + ops_by_addr - 1;
                for index in 0..addr_count {
                    self.addr
                        .entry(addr_w + index)
                        .and_modify(|value| {
                            value.count += ops_by_addr +
                                Self::count_extra_internal_reads(value.last_step, step);
                            value.last_step = last_step;
                        })
                        .or_insert(UsesCounter { first_step: step, last_step, count: ops_by_addr });
                    addr_w += 1;
                }
                let mem_align_op_rows = 1 + addr_count * ops_by_addr as u32;
                self.mem_align.push(mem_align_op_rows as u8);
                self.mem_align_rows += mem_align_op_rows;
            }
        }

        vec![]
    }

    fn add(&mut self, _other: &dyn Metrics) {}

    fn op_type(&self) -> Vec<ZiskOperationType> {
        vec![zisk_core::ZiskOperationType::Arith]
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl BusDevice<u64> for MemCounters {
    #[inline]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        self.measure(bus_id, data);

        vec![]
    }
}
