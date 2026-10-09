//! The single bus device a `CompactMem` instance gets per chunk.
//!
//! One collector per (instance, chunk) is all the collect phase can hand back -- `collectors_by
//! _instance` keeps exactly one slot per chunk of an instance -- but a `CompactMem` instance is
//! three memory areas at once, and each area filters the bus by its own address range. So the three
//! `MemModuleCollector`s it needs travel together inside this one device, which does nothing but
//! forward.

use crate::MemModuleCollector;
use zisk_common::{BusDevice, BusId};
use zisk_sm_mem_common::MemArea;

#[derive(Debug)]
pub struct CompactMemCollector {
    /// The areas that have something to collect in this chunk, in `MemArea::ALL` order. An area
    /// whose segment does not reach this chunk is simply absent.
    collectors: Vec<(MemArea, MemModuleCollector)>,
}

impl CompactMemCollector {
    pub fn new(collectors: Vec<(MemArea, MemModuleCollector)>) -> Self {
        Self { collectors }
    }

    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        for (_, collector) in self.collectors.iter_mut() {
            collector.process_data(bus_id, data);
        }
        true
    }

    /// An address can only be skipped when every area would skip it.
    pub fn skip_addr(&self, addr: u32) -> bool {
        self.collectors.iter().all(|(_, collector)| collector.skip_addr(addr))
    }

    /// Likewise for a range.
    pub fn skip_addr_range(&self, addr_from: u32, addr_to: u32) -> bool {
        self.collectors.iter().all(|(_, collector)| collector.skip_addr_range(addr_from, addr_to))
    }

    /// Hands the per-area collectors over to the witness computation.
    pub fn into_collectors(self) -> Vec<(MemArea, MemModuleCollector)> {
        self.collectors
    }
}

impl BusDevice<u64> for CompactMemCollector {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
