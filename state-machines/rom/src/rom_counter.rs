use std::any::Any;

use sm_common::{CounterStats, Metrics};
use zisk_common::{BusDevice, BusId, RomBusData, ROM_BUS_DATA_SIZE};
use zisk_core::ZiskOperationType;

pub struct RomCounter {
    bus_id: BusId,
    pub rom: CounterStats,
    pub end_pc: u64,
    pub steps: u64,
}

impl RomCounter {
    pub fn new(bus_id: BusId) -> Self {
        Self { bus_id, rom: CounterStats::default(), end_pc: 0, steps: 0 }
    }
}

impl Metrics for RomCounter {
    fn measure(&mut self, _: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        let data: &[u64; ROM_BUS_DATA_SIZE] =
            data.try_into().expect("Rom Metrics: Failed to convert data");
        let inst_pc = RomBusData::get_pc(data);
        let inst_step = RomBusData::get_step(data);
        let inst_end = RomBusData::get_end(data);

        self.rom.update(inst_pc, 1);
        if inst_end == 1 {
            self.end_pc = inst_pc;
            self.steps = inst_step + 1;
        }

        vec![]
    }

    fn add(&mut self, other: &dyn Metrics) {
        let other =
            other.as_any().downcast_ref::<RomCounter>().expect("Rom Metrics: Failed to downcast");
        self.rom += &other.rom;

        if other.end_pc != 0 {
            self.end_pc = other.end_pc;
        }

        if other.steps != 0 {
            self.steps = other.steps;
        }
    }

    fn op_type(&self) -> Vec<ZiskOperationType> {
        vec![ZiskOperationType::None]
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
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        self.measure(bus_id, data);

        vec![]
    }
}
