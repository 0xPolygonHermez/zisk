//! The single bus device a `CompactDma` instance gets per chunk.
//!
//! The collect phase keeps exactly one collector per (instance, chunk), but a `CompactDma`
//! instance is two airs at once, and each one filters the bus by its own plan. So the two
//! collectors it needs travel together inside this one device, which does nothing but forward.

use std::any::Any;

use zisk_common::{BusDevice, BusId};

use crate::{DmaLoopCollector, DmaWithPrePostCollector};

pub struct CompactDmaCollector {
    /// The `wpp_` block's collector, when that block has something to collect in this chunk.
    pub wpp: Option<DmaWithPrePostCollector>,
    /// The `loop_` block's collector, likewise.
    pub lp: Option<DmaLoopCollector>,
}

impl CompactDmaCollector {
    pub fn new(wpp: Option<DmaWithPrePostCollector>, lp: Option<DmaLoopCollector>) -> Self {
        Self { wpp, lp }
    }

    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64], data_ext: &[u64]) -> bool {
        if let Some(wpp) = self.wpp.as_mut() {
            wpp.process_data(bus_id, data, data_ext);
        }
        if let Some(lp) = self.lp.as_mut() {
            lp.process_data(bus_id, data, data_ext);
        }
        true
    }

    pub fn get_debug_info(&self) -> String {
        self.wpp.as_ref().map_or(String::new(), |c| c.get_debug_info())
            + &self.lp.as_ref().map_or(String::new(), |c| c.get_debug_info())
    }
}

impl BusDevice<u64> for CompactDmaCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
