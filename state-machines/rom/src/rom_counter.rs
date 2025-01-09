//! The `RomCounter` module defines a counter for tracking ROM execution metrics
//! sent over the data bus. It collects statistics such as program counter (PC),
//! executed instruction steps, and the PC of the last executed instruction.

use std::any::Any;

use sm_common::{CounterStats, Metrics};
use zisk_common::{BusDevice, BusId, RomBusData, RomData};

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

impl Metrics for RomCounter {
    /// Tracks activity on the connected bus and updates ROM execution metrics.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (ignored in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs.
    fn measure(&mut self, _: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        let data: RomData<u64> = data.try_into().expect("Rom Metrics: Failed to convert data");

        self.rom.update(
            RomBusData::get_pc(&data),
            RomBusData::get_step(&data),
            1,
            RomBusData::get_end(&data) == 1,
        );

        vec![]
    }

    /// Merges metrics from another `RomCounter`.
    ///
    /// # Arguments
    /// * `other` - A reference to another `Metrics` instance that must be a `RomCounter`.
    ///
    /// # Panics
    /// Panics if the `other` is not of type `RomCounter`.
    fn add(&mut self, other: &dyn Metrics) {
        let other =
            other.as_any().downcast_ref::<RomCounter>().expect("Rom Metrics: Failed to downcast");
        self.rom += &other.rom;
    }

    /// Returns the bus IDs associated with this counter.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
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
    /// A tuple where:
    /// - The first element is `true`, indicating that processing should continue.
    /// - The second element is an empty vector, as this implementation does not produce derived inputs.
    #[inline]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        self.measure(bus_id, data);

        (true, vec![])
    }
}
