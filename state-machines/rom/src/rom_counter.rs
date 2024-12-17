use std::any::Any;

use sm_common::{CounterStats, Metrics};
use zisk_common::{BusDevice, DataBusMain, Opid};
use zisk_core::ZiskOperationType;

#[derive(Default)]
pub struct RomCounter {
    pub rom: CounterStats,
    pub end_pc: u64,
    pub steps: u64,
}

impl Metrics for RomCounter {
    fn measure(&mut self, opid: &Opid, data: &[u64]) -> Vec<(Opid, Vec<u64>)> {
        if *opid == 5000 {
            let data: &[u64; 8] = data.try_into().expect("Regular Metrics: Failed to convert data");
            let inst_pc = DataBusMain::get_pc(data);
            let inst_step = DataBusMain::get_step(data);
            let inst_end = DataBusMain::get_end(data);

            self.rom.update(inst_pc, 1);
            if inst_end == 1 {
                self.end_pc = inst_pc;
                self.steps = inst_step + 1;
            }
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

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BusDevice<u64> for RomCounter {
    #[inline]
    fn process_data(&mut self, opid: &Opid, data: &[u64]) -> Vec<(Opid, Vec<u64>)> {
        self.measure(opid, data);

        vec![]
    }
}
