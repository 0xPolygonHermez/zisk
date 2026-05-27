//! The `RomCounter` module defines a counter for tracking ROM execution metrics
//! sent over the data bus. It collects statistics such as program counter (PC),
//! executed instruction steps, and the PC of the last executed instruction.

use std::{
    any::Any,
    sync::{atomic::AtomicU64, Arc},
};

use zisk_common::{CounterStats, Metrics, RomBusData, RomData};

/// The `RomCounter` struct represents a counter that monitors ROM-related metrics
/// on the data bus.
///
/// It collects execution statistics, such as the program counter (PC) of executed instructions,
/// the total number of executed steps, and the PC of the last executed instruction.
pub(crate) struct RomCounter {
    /// Execution statistics counter for ROM instructions.
    pub(crate) counter_stats: CounterStats,
}

impl RomCounter {
    /// Creates a new instance of `RomCounter`.
    ///
    /// # Returns
    /// A new `RomCounter` instance.
    pub(crate) fn new(inst_count: Arc<Vec<AtomicU64>>) -> Self {
        let counter_stats = CounterStats::new(inst_count);
        Self { counter_stats }
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
            RomBusData::get_index(&data),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    fn atomics_of_len(n: usize) -> Arc<Vec<AtomicU64>> {
        Arc::new((0..n).map(|_| AtomicU64::new(0)).collect())
    }

    /// ROM bus payload layout (from `RomBusData<u64>`): `[step, pc, index, end]`.
    fn rom_bus_data(step: u64, pc: u64, index: u64, end: u64) -> [u64; 4] {
        [step, pc, index, end]
    }

    #[test]
    fn measure_increments_inst_count_at_index() {
        let inst_count = atomics_of_len(8);
        let mut counter = RomCounter::new(inst_count.clone());

        counter.measure(&rom_bus_data(0, 0x8000_0000, 3, 0));
        assert_eq!(inst_count[3].load(Ordering::Relaxed), 1);

        counter.measure(&rom_bus_data(1, 0x8000_0004, 3, 0));
        assert_eq!(inst_count[3].load(Ordering::Relaxed), 2);
    }

    #[test]
    fn measure_with_end_flag_updates_end_pc_and_steps() {
        let mut counter = RomCounter::new(atomics_of_len(8));

        counter.measure(&rom_bus_data(42, 0x8000_0010, 1, /* end */ 1));

        assert_eq!(counter.counter_stats.end_pc, 0x8000_0010);
        assert_eq!(counter.counter_stats.steps, 43, "steps = step + 1 when end is set");
    }

    #[test]
    fn measure_without_end_flag_leaves_end_pc_default() {
        let mut counter = RomCounter::new(atomics_of_len(8));

        counter.measure(&rom_bus_data(10, 0x8000_0000, 0, /* end */ 0));
        counter.measure(&rom_bus_data(20, 0x8000_0004, 1, /* end */ 0));

        assert_eq!(counter.counter_stats.end_pc, 0);
        assert_eq!(counter.counter_stats.steps, 0);
    }
}
