use std::sync::Arc;

use crate::{
    DummyMemPlanner, InputDataSM, MemAlignByteInstance, MemAlignByteSM, MemAlignInstance,
    MemAlignReadByteInstance, MemAlignSM, MemAlignWriteByteInstance, MemModuleInstance, MemPlanner,
    MemSM, RomDataSM,
};
use fields::PrimeField64;
use mem_common::MemCounters;
use pil_std_lib::Std;
use proofman_common::ProofCtx;
use zisk_common::{BusDeviceMetrics, ComponentBuilder, Instance, InstanceCtx, Plan, Planner};
use zisk_pil::{
    InputDataTrace, MemAlignByteTrace, MemAlignReadByteTrace, MemAlignTrace,
    MemAlignWriteByteTrace, MemTrace, RomDataTrace, ZiskProofValues,
};

pub struct Mem<F: PrimeField64> {
    // Secondary State machines
    mem_sm: Arc<MemSM<F>>,
    mem_align_sm: Arc<MemAlignSM<F>>,
    mem_align_byte_sm: Arc<MemAlignByteSM<F>>,
    input_data_sm: Arc<InputDataSM<F>>,
    rom_data_sm: Arc<RomDataSM<F>>,
}

impl<F: PrimeField64> Mem<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let mem_align_sm = MemAlignSM::new(std.clone());
        let mem_sm = MemSM::new(std.clone());
        let input_data_sm = InputDataSM::new(std.clone());
        let rom_data_sm = RomDataSM::new(std.clone());
        let mem_align_byte_sm = MemAlignByteSM::new(std.clone());

        Arc::new(Self { mem_align_sm, mem_sm, input_data_sm, rom_data_sm, mem_align_byte_sm })
    }

    pub fn build_mem_counter(&self) -> MemCounters {
        MemCounters::new()
    }

    // This method is used to create a dummy planner when using count-and-plan in C++
    pub fn build_dummy_planner(&self) -> Box<dyn Planner> {
        Box::new(DummyMemPlanner::new())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for Mem<F> {
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(MemCounters::new()))
    }

    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(MemPlanner::new())
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Plan]) {
        let enable_input_data = plannings.iter().any(|p| p.air_id == InputDataTrace::<F>::AIR_ID);
        let mut proof_values = ZiskProofValues::from_vec_guard(pctx.get_proof_values());
        proof_values.enable_input_data = F::from_bool(enable_input_data);
    }

    /// Builds an instance of the Memory state machine.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///
    /// # Returns
    /// A boxed implementation of a Memory Instance.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            MemTrace::<F>::AIR_ID => Box::new(MemModuleInstance::new(self.mem_sm.clone(), ictx)),
            RomDataTrace::<F>::AIR_ID => {
                Box::new(MemModuleInstance::new(self.rom_data_sm.clone(), ictx))
            }
            InputDataTrace::<F>::AIR_ID => {
                Box::new(MemModuleInstance::new(self.input_data_sm.clone(), ictx))
            }
            MemAlignTrace::<F>::AIR_ID => {
                Box::new(MemAlignInstance::new(self.mem_align_sm.clone(), ictx))
            }
            MemAlignByteTrace::<F>::AIR_ID => {
                Box::new(MemAlignByteInstance::new(self.mem_align_byte_sm.clone(), ictx))
            }
            MemAlignReadByteTrace::<F>::AIR_ID => {
                Box::new(MemAlignReadByteInstance::new(self.mem_align_byte_sm.clone(), ictx))
            }
            MemAlignWriteByteTrace::<F>::AIR_ID => {
                Box::new(MemAlignWriteByteInstance::new(self.mem_align_byte_sm.clone(), ictx))
            }
            _ => panic!("Memory::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }
}
