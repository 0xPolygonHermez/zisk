use rayon::prelude::*;
use std::collections::HashMap;

use zisk_common::{BusDevice, BusId, MemBusData, Metrics, MEM_BUS_ID};

use crate::MemHelpers;

#[cfg(feature = "debug_mem")]
use crate::MemDebug;

#[cfg(feature = "debug_mem")]
#[derive(Debug, Clone, Copy, Default)]
pub struct UsesCounterDebug {
    pub internal_reads: u32,
    pub mem_align_extra_rows: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UsesCounter {
    pub first_step: u64,
    pub last_step: u64,
    pub count: u64,
    #[cfg(feature = "debug_mem")]
    pub debug: UsesCounterDebug,
}

#[derive(Default, Debug)]
pub struct MemCounters {
    pub addr: HashMap<u32, UsesCounter>,
    pub addr_sorted: [Vec<(u32, UsesCounter)>; 3],
    pub mem_align: Vec<u8>,
    pub mem_align_rows: u32,
    #[cfg(feature = "debug_mem")]
    pub debug: MemDebug,
}

impl MemCounters {
    pub fn new() -> Self {
        Self {
            addr: HashMap::new(),
            addr_sorted: [Vec::new(), Vec::new(), Vec::new()],
            mem_align: Vec::new(),
            mem_align_rows: 0,
            #[cfg(feature = "debug_mem")]
            debug: MemDebug::new(),
        }
    }
}

impl Metrics for MemCounters {
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        let op = MemBusData::get_op(data);
        let addr = MemBusData::get_addr(data);
        let addr_w = MemHelpers::get_addr_w(addr);
        let step = MemBusData::get_step(data);
        let bytes = MemBusData::get_bytes(data);

        // #[cfg(feature = "debug_mem")]
        // self.debug.log(addr, step, bytes, is_write, false);

        if MemHelpers::is_aligned(addr, bytes) {
            self.addr
                .entry(addr_w)
                .and_modify(|value| {
                    let internal_reads =
                        MemHelpers::get_extra_internal_reads_by_addr(addr_w, value.last_step, step);
                    value.count += 1 + internal_reads;
                    #[cfg(feature = "debug_mem")]
                    {
                        value.debug.internal_reads += internal_reads as u32;
                    }
                    value.last_step = step;
                })
                .or_insert(UsesCounter {
                    first_step: step,
                    last_step: step,
                    count: 1,
                    #[cfg(feature = "debug_mem")]
                    debug: UsesCounterDebug { internal_reads: 0, mem_align_extra_rows: 0 },
                });
        } else {
            let addr_count = if MemHelpers::is_double(addr, bytes) { 2 } else { 1 };
            let ops_by_addr = if MemHelpers::is_write(op) { 2 } else { 1 };

            let last_step = step + ops_by_addr - 1;
            for index in 0..addr_count {
                let _addr_w = addr_w + index; // addr_w, addr_w + 1
                self.addr
                    .entry(_addr_w)
                    .and_modify(|value| {
                        let internal_reads = MemHelpers::get_extra_internal_reads_by_addr(
                            _addr_w,
                            value.last_step,
                            step,
                        );
                        value.count += ops_by_addr + internal_reads;
                        value.last_step = last_step;
                        #[cfg(feature = "debug_mem")]
                        {
                            value.debug.internal_reads += internal_reads as u32;
                            value.debug.mem_align_extra_rows += ops_by_addr as u32 - 1;
                        }
                    })
                    .or_insert(UsesCounter {
                        first_step: step,
                        last_step,
                        count: ops_by_addr,
                        #[cfg(feature = "debug_mem")]
                        debug: UsesCounterDebug {
                            internal_reads: 0,
                            mem_align_extra_rows: ops_by_addr as u32 - 1,
                        },
                    });
            }
            let mem_align_op_rows = 1 + addr_count * ops_by_addr as u32;
            self.mem_align.push(mem_align_op_rows as u8);
            self.mem_align_rows += mem_align_op_rows;
        }
    }

    fn on_close(&mut self) {
        // address must be ordered
        let mut addr_vector: Vec<(u32, UsesCounter)> =
            std::mem::take(&mut self.addr).into_iter().collect();
        addr_vector.par_sort_by_key(|(key, _)| *key);

        // Divideix el vector original en tres parts
        let point = addr_vector.partition_point(|x| x.0 < (0xA000_0000 / 8));
        self.addr_sorted[2] = addr_vector.split_off(point);

        let point = addr_vector.partition_point(|x| x.0 < (0x9000_0000 / 8));
        self.addr_sorted[1] = addr_vector.split_off(point);

        self.addr_sorted[0] = addr_vector;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl BusDevice<u64> for MemCounters {
    #[inline]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        debug_assert!(bus_id == &MEM_BUS_ID);

        self.measure(data);

        None
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
