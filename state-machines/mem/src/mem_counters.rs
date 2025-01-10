use std::collections::HashMap;

use sm_common::Metrics;
use zisk_common::{BusDevice, BusId, MEM_BUS_ID};

use crate::{MemHelpers, MEM_BYTES_BITS, MEM_REGS_ADDR, MEM_REGS_MASK};

use log::info;

#[derive(Debug, Clone, Copy, Default)]
pub struct UsesCounter {
    pub first_step: u64,
    pub last_step: u64,
    pub count: u64,
    pub last_value: u64,
}

#[derive(Default)]
pub struct MemCounters {
    pub registers: [UsesCounter; 32],
    pub addr: HashMap<u32, UsesCounter>,
    pub addr_sorted: Vec<(u32, UsesCounter)>,
    pub mem_align: Vec<u8>,
    pub mem_align_rows: u32,
}

impl MemCounters {
    pub fn new() -> Self {
        let empty_counter = UsesCounter::default();
        Self {
            registers: [empty_counter; 32],
            addr: HashMap::new(),
            addr_sorted: Vec::new(),
            mem_align: Vec::new(),
            mem_align_rows: 0,
        }
    }
}

impl Metrics for MemCounters {
    fn measure(&mut self, _: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        // info!("[Mem]   MemCounters::measure....");
        let op = data[0] as u8;
        let addr = data[1] as u32;
        let mut addr_w = addr >> MEM_BYTES_BITS;
        let step = data[2];
        let bytes = data[3] as u8;

        if (addr & MEM_REGS_MASK) == MEM_REGS_ADDR {
            let reg_index = ((addr >> 3) & 0x1F) as usize;
            if self.registers[reg_index].count == 0 {
                self.registers[reg_index] = UsesCounter {
                    first_step: step,
                    last_step: step,
                    count: 1,
                    last_value: data[4],
                };
            } else {
                // TODO: this only applies to non-imputable memories (mem)
                self.registers[reg_index].count += 1 + MemHelpers::get_extra_internal_reads(
                    self.registers[reg_index].last_step,
                    step,
                );
                self.registers[reg_index].last_step = step;
                self.registers[reg_index].last_value = data[4];
            }
        } else {
            let aligned = addr & 0x7 == 0 && bytes == 8;
            // TODO: last value must be calculated as last value operation
            // R: value[4]
            // RR:
            let last_value = data[4];
            if aligned {
                // TODO: read, write
                self.addr
                    .entry(addr_w)
                    .and_modify(|value| {
                        value.count += 1;
                        value.last_step = step;
                        value.last_value = last_value;
                    })
                    .or_insert(UsesCounter {
                        first_step: step,
                        last_step: step,
                        count: 1,
                        last_value,
                    });
            } else {
                // TODO: use mem_align helpers
                // TODO: last value must be calculated as last value operation
                let last_value = 0;
                let addr_count = if MemHelpers::is_double(addr, bytes) { 2 } else { 1 };
                let ops_by_addr = if MemHelpers::is_write(op) { 2 } else { 1 };

                let last_step = step + ops_by_addr - 1;
                for index in 0..addr_count {
                    self.addr
                        .entry(addr_w + index)
                        .and_modify(|value| {
                            value.count += ops_by_addr +
                                MemHelpers::get_extra_internal_reads_by_addr(
                                    addr_w + index,
                                    value.last_step,
                                    step,
                                );
                            value.last_step = last_step;
                            value.last_value = last_value
                        })
                        .or_insert(UsesCounter {
                            first_step: step,
                            last_step,
                            count: ops_by_addr,
                            last_value,
                        });
                    // if addr_count > 1, then addr_w must be the next (addr_w is expressed in
                    // MEM_BYTES)
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

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }
    fn on_close(&mut self) {
        // address must be ordered
        info!("[Mem]   Closing....");
        let addr_hashmap = std::mem::take(&mut self.addr);
        self.addr_sorted = addr_hashmap.into_iter().collect();
        self.addr_sorted.sort_by(|a, b| a.0.cmp(&b.0));
        info!("[Mem]   Closed");
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl BusDevice<u64> for MemCounters {
    #[inline]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        self.measure(bus_id, data);

        (false, vec![])
    }
}
