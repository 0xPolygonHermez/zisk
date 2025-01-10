use std::sync::Arc;

use crate::{
    InputDataSM, MemAlignInstance, MemAlignRomSM, MemAlignSM, MemCounters, MemModuleInstance,
    MemPlanner, MemSM, RomDataSM,
};
use log::info;
use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{
    table_instance, BusDeviceInstance, BusDeviceMetrics, ComponentBuilder, InstanceCtx, Planner,
};
use zisk_common::MEM_BUS_ID;
use zisk_pil::{InputDataTrace, MemAlignRomTrace, MemAlignTrace, MemTrace, RomDataTrace};

pub struct MemProxy<F: PrimeField> {
    // Secondary State machines
    mem_sm: Arc<MemSM<F>>,
    mem_align_sm: Arc<MemAlignSM<F>>,
    mem_align_rom_sm: Arc<MemAlignRomSM>,
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

        Arc::new(Self { mem_align_sm, mem_align_rom_sm, mem_sm, input_data_sm, rom_data_sm })
    }
}

impl<F: PrimeField> ComponentBuilder<F> for MemProxy<F> {
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        info!("[Mem]   MemProxy::build_counter");
        Box::new(MemCounters::new())
        // Box::new(MemCounters::new(OPERATION_BUS_ID, vec![zisk_core::ZiskOperationType::Arith]))
    }

    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(MemPlanner::new())
    }

    fn build_inputs_collector(&self, ictx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>> {
        match ictx.plan.air_id {
            id if id == MemTrace::<usize>::AIR_ID => {
                Box::new(MemModuleInstance::new(self.mem_sm.clone(), ictx))
            }
            id if id == RomDataTrace::<usize>::AIR_ID => {
                Box::new(MemModuleInstance::new(self.rom_data_sm.clone(), ictx))
            }
            id if id == InputDataTrace::<usize>::AIR_ID => {
                Box::new(MemModuleInstance::new(self.input_data_sm.clone(), ictx))
            }
            id if id == MemAlignTrace::<usize>::AIR_ID => {
                Box::new(MemAlignInstance::new(self.mem_align_sm.clone(), ictx))
            }
            id if id == MemAlignRomTrace::<usize>::AIR_ID => {
                table_instance!(MemAlignRomInstance, MemAlignRomSM, MemAlignRomTrace);
                Box::new(MemAlignRomInstance::new(self.mem_align_rom_sm.clone(), ictx, MEM_BUS_ID))
            }
            _ => panic!("Memory::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }
    fn build_inputs_generator(&self) -> Option<Box<dyn BusDeviceInstance<F>>> {
        None
    }
}
