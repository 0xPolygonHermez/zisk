//! The single bus device a `CompactBinary` instance gets per chunk.
//!
//! One collector per (instance, chunk) is all the collect phase can hand back, but a `CompactBinary`
//! instance is four airs at once, each filtering the bus by its own kinds. So the four standalone
//! collectors it needs travel together inside this one device, which does nothing but route: the
//! basic-family operations to the three add-family collectors, the extension ones to the fourth.
//! Each of them replays the chunk exactly as it does on its own air.

use crate::{
    BinaryAddCollector, BinaryAddHiCollector, BinaryBasicCollector, BinaryExtensionCollector,
};
use proofman_fields::PrimeField64;
use zisk_common::{BusDevice, BusId, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::ZiskOperationType;

pub struct CompactBinaryCollector<F: PrimeField64> {
    /// The blocks that have something to collect in this chunk. A block whose plan does not reach
    /// this chunk is simply absent.
    pub basic: Option<BinaryBasicCollector<F>>,
    pub add: Option<BinaryAddCollector<F>>,
    pub add_hi: Option<BinaryAddHiCollector<F>>,
    pub ext: Option<BinaryExtensionCollector<F>>,
}

impl<F: PrimeField64> CompactBinaryCollector<F> {
    /// Routes one operation of the bus to the blocks that see its type. Returns `false` once no
    /// block wants anything else from the chunk, which is what lets the replay stop early; a block
    /// that is done keeps answering `false` on its own, so it costs nothing to keep asking it.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);
        const BINARY: u64 = ZiskOperationType::Binary as u64;
        const BINARY_E: u64 = ZiskOperationType::BinaryE as u64;

        let mut more = false;
        match data[OP_TYPE] {
            BINARY => {
                if let Some(c) = self.add.as_mut() {
                    more |= c.process_data(bus_id, data);
                }
                if let Some(c) = self.add_hi.as_mut() {
                    more |= c.process_data(bus_id, data);
                }
                if let Some(c) = self.basic.as_mut() {
                    more |= c.process_data(bus_id, data);
                }
                // The extension block still has to see the end of the chunk when it has anything
                // left to take there.
                more |= self.ext.is_some();
            }
            BINARY_E => {
                if let Some(c) = self.ext.as_mut() {
                    more |= c.process_data(bus_id, data);
                }
                more |= self.basic.is_some() || self.add.is_some() || self.add_hi.is_some();
            }
            _ => more = true,
        }
        more
    }
}

impl<F: PrimeField64> BusDevice<u64> for CompactBinaryCollector<F> {
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
