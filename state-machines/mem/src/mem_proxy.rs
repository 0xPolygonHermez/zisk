use std::sync::Arc;

use crate::{
    InputDataSM, MemAlignRomSM, MemAlignSM, MemCounters, MemProxyEngine, MemSM, RomDataSM,
};
use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{
    BusDeviceInstance, BusDeviceMetrics, ComponentProvider, InstanceExpanderCtx, Planner,
};
use zisk_core::ZiskRequiredMemory;

pub struct MemProxy<F: PrimeField> {
    // Secondary State machines
    mem_sm: Arc<MemSM<F>>,
    mem_align_sm: Arc<MemAlignSM<F>>,
    _mem_align_rom_sm: Arc<MemAlignRomSM>,
    input_data_sm: Arc<InputDataSM<F>>,
    rom_data_sm: Arc<RomDataSM<F>>,
}

impl<F: PrimeField> MemProxy<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let mem_align_rom_sm = MemAlignRomSM::new();
        let mem_align_sm = MemAlignSM::new(std.clone(), mem_align_rom_sm.clone());
        let mem_sm = MemSM::new(std.clone());
        let input_data_sm = InputDataSM::new(std.clone());
        let rom_data_sm = RomDataSM::new(std.clone());

        Arc::new(Self {
            mem_align_sm,
            _mem_align_rom_sm: mem_align_rom_sm,
            mem_sm,
            input_data_sm,
            rom_data_sm,
        })
    }

    pub fn prove(
        &self,
        mem_operations: &mut Vec<ZiskRequiredMemory>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut engine = MemProxyEngine::<F>::new(self.mem_align_sm.clone());
        engine.add_module("mem", self.mem_sm.clone());
        engine.add_module("input_data", self.input_data_sm.clone());
        engine.add_module("row_data", self.rom_data_sm.clone());
        engine.prove(mem_operations)
    }
}

impl<F: PrimeField> ComponentProvider<F> for MemProxy<F> {
    fn get_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(MemCounters::new())
        // Box::new(MemCounters::new(OPERATION_BUS_ID, vec![zisk_core::ZiskOperationType::Arith]))
    }

    fn get_planner(&self) -> Box<dyn Planner> {
        unimplemented!("get_planner for MemProxy");
    }

    fn get_instance(&self, iectx: InstanceExpanderCtx) -> Box<dyn BusDeviceInstance<F>> {
        unimplemented!("get_instance for MemProxy");
    }
    fn get_inputs_generator(&self) -> Option<Box<dyn BusDeviceInstance<F>>> {
        unimplemented!("get_instance for MemProxy");
    }
}
