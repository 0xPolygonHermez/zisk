//! The `RomCounter` module defines a counter for tracking ROM execution metrics
//! sent over the data bus. It collects statistics such as program counter (PC),
//! executed instruction steps, and the PC of the last executed instruction.

use std::{
    any::Any,
    ops::AddAssign,
    sync::{atomic::AtomicU32, Arc},
};

use zisk_common::{BusDevice, BusId, CounterStats, Metrics, RomBusData, RomData, ROM_BUS_ID};

/// The `RomCounter` struct represents a counter that monitors ROM-related metrics
/// on the data bus.
///
/// It collects execution statistics, such as the program counter (PC) of executed instructions,
/// the total number of executed steps, and the PC of the last executed instruction.
pub struct RomCounter {
    /// Execution statistics counter for ROM instructions.
    pub counter_stats: CounterStats,
}

impl RomCounter {
    /// Creates a new instance of `RomCounter`.
    ///
    /// # Returns
    /// A new `RomCounter` instance.
    pub fn new(bios_inst_count: Arc<Vec<AtomicU32>>, prog_inst_count: Arc<Vec<AtomicU32>>) -> Self {
        let counter_stats = CounterStats::new(bios_inst_count, prog_inst_count);
        Self { counter_stats }
    }
}

impl AddAssign<&RomCounter> for RomCounter {
    fn add_assign(&mut self, other: &Self) {
        self.counter_stats += &other.counter_stats;
    }
}

impl Metrics for RomCounter {
    /// Tracks activity on the connected bus and updates ROM execution metrics.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (ignored in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs.
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        let data: RomData<u64> = data.try_into().expect("Rom Metrics: Failed to convert data");

        self.counter_stats.update(
            RomBusData::get_pc(&data),
            RomBusData::get_step(&data),
            1,
            RomBusData::get_end(&data) == 1,
        );
    }

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BusDevice<u64> for RomCounter {
    /// Processes data received on the bus, updating ROM metrics.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An optional vector of tuples where:
    /// - The first element is the bus ID.
    /// - The second element is always empty indicating there are no derived inputs.
    #[inline]
    fn process_data(&mut self, _bus_id: &BusId, _data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        None
    }

    /// Returns the bus IDs associated with this counter.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![ROM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
