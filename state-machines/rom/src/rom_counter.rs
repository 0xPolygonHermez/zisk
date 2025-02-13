//! The `RomCounter` module defines a counter for tracking ROM execution metrics
//! sent over the data bus. It collects statistics such as program counter (PC),
//! executed instruction steps, and the PC of the last executed instruction.

use std::{any::Any, ops::AddAssign};

use data_bus::{BusDevice, BusId, RomBusData, RomData};
use sm_common::{CounterStats, Metrics};

/// The `RomCounter` struct represents a counter that monitors ROM-related metrics
/// on the data bus.
///
/// It collects execution statistics, such as the program counter (PC) of executed instructions,
/// the total number of executed steps, and the PC of the last executed instruction.
pub struct RomCounter {
    /// The connected bus ID.
    bus_id: BusId,

    /// Execution statistics counter for ROM instructions.
    pub rom: CounterStats,
}

impl RomCounter {
    /// Creates a new instance of `RomCounter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    ///
    /// # Returns
    /// A new `RomCounter` instance.
    pub fn new(bus_id: BusId) -> Self {
        Self { bus_id, rom: CounterStats::default() }
    }
}

impl AddAssign<&RomCounter> for RomCounter {
    fn add_assign(&mut self, other: &Self) {
        self.rom += &other.rom; // Directly add `other.rom` to `self.rom`
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
    fn measure(&mut self, data: &[u64]) {
        let data: RomData<u64> = data.try_into().expect("Rom Metrics: Failed to convert data");

        self.rom.update(
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
    fn process_data(&mut self, _: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        self.measure(data);

        None
    }

    /// Returns the bus IDs associated with this counter.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
