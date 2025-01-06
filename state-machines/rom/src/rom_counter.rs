use std::any::Any;

use sm_common::{CounterStats, Metrics};
use zisk_common::{BusDevice, BusId, RomBusData, RomData};

pub struct RomCounter {
    /// Bus Id
    bus_id: BusId,

    /// Execution Statistics counter
    pub rom: CounterStats,
}

impl RomCounter {
    pub fn new(bus_id: BusId) -> Self {
        Self { bus_id, rom: CounterStats::default() }
    }
}

impl Metrics for RomCounter {
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

    fn add(&mut self, other: &dyn Metrics) {
        let other =
            other.as_any().downcast_ref::<RomCounter>().expect("Rom Metrics: Failed to downcast");
        self.rom += &other.rom;
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BusDevice<u64> for RomCounter {
    #[inline]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        self.measure(bus_id, data);

        (true, vec![])
    }
}
