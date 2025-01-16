use std::collections::HashMap;

use sm_common::Metrics;
use zisk_common::{BusDevice, BusId, MemBusData, MEM_BUS_ID};

use crate::{MemHelpers, MEM_REGS_ADDR, MEM_REGS_MASK};

#[derive(Debug, Clone, Copy, Default)]
pub struct UsesCounter {
    pub first_step: u64,
    pub last_step: u64,
    pub count: u64,
    pub last_value: u64,
}

#[derive(Default, Debug)]
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
        let op = MemBusData::get_op(data);
        let is_write = MemHelpers::is_write(op);
        let addr = MemBusData::get_addr(data);
        let addr_w = MemHelpers::get_addr_w(addr);
        let step = MemBusData::get_step(data);
        let bytes = MemBusData::get_bytes(data);

        if (addr & MEM_REGS_MASK) == MEM_REGS_ADDR {
            let reg_index = ((addr >> 3) & 0x1F) as usize;
            // last value is needed for continuations and internal reads
            let last_value = if is_write {
                MemBusData::get_value(data)
            } else {
                MemBusData::get_mem_values(data)[0]
            };

            if self.registers[reg_index].count == 0 {
                self.registers[reg_index] =
                    UsesCounter { first_step: step, last_step: step, count: 1, last_value };
            } else {
                // TODO: this only applies to non-imputable memories (mem)
                self.registers[reg_index].count += 1 + MemHelpers::get_extra_internal_reads(
                    self.registers[reg_index].last_step,
                    step,
                );
                self.registers[reg_index].last_step = step;
                self.registers[reg_index].last_value = last_value;
            }
        } else if MemHelpers::is_aligned(addr, bytes) {
            let last_value = if is_write {
                MemBusData::get_value(data)
            } else {
                MemBusData::get_mem_values(data)[0]
            };
            self.addr
                .entry(addr_w)
                .and_modify(|value| {
                    value.count += 1 + MemHelpers::get_extra_internal_reads_by_addr(
                        addr_w,
                        value.last_step,
                        step,
                    );
                    value.last_step = step;
                    value.last_value = last_value;
                })
                .or_insert(UsesCounter { first_step: step, last_step: step, count: 1, last_value });
        } else {
            let addr_count = if MemHelpers::is_double(addr, bytes) { 2 } else { 1 };
            let (ops_by_addr, last_values) = if MemHelpers::is_write(op) {
                (
                    2,
                    MemHelpers::get_write_values(
                        addr_w,
                        bytes,
                        MemBusData::get_value(data),
                        MemBusData::get_mem_values(data),
                    ),
                )
            } else {
                (1, MemBusData::get_mem_values(data))
            };

            let last_step = step + ops_by_addr - 1;
            for index in 0..addr_count {
                let _addr_w = addr_w + index; // addr_w, addr_w + 1
                self.addr
                    .entry(_addr_w)
                    .and_modify(|value| {
                        value.count += ops_by_addr +
                            MemHelpers::get_extra_internal_reads_by_addr(
                                _addr_w,
                                value.last_step,
                                step,
                            );
                        value.last_step = last_step;
                        value.last_value = last_values[index as usize];
                    })
                    .or_insert(UsesCounter {
                        first_step: step,
                        last_step,
                        count: ops_by_addr,
                        last_value: last_values[index as usize],
                    });
            }
            let mem_align_op_rows = 1 + addr_count * ops_by_addr as u32;
            self.mem_align.push(mem_align_op_rows as u8);
            self.mem_align_rows += mem_align_op_rows;
        }

        vec![]
    }

    fn add(&mut self, _other: &dyn Metrics) {}

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }
    fn on_close(&mut self) {
        // address must be ordered
        let addr_hashmap = std::mem::take(&mut self.addr);
        self.addr_sorted = addr_hashmap.into_iter().collect();
        self.addr_sorted.sort_by(|a, b| a.0.cmp(&b.0));
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
