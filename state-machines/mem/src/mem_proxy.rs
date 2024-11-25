use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use crate::{InputDataSM, MemAlignRomSM, MemAlignSM, MemProxyEngine, MemSM};
use p3_field::PrimeField;
use pil_std_lib::Std;
use zisk_core::ZiskRequiredMemory;

use proofman::{WitnessComponent, WitnessManager};

pub struct MemProxy<F: PrimeField> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Secondary State machines
    mem_sm: Arc<MemSM<F>>,
    mem_align_sm: Arc<MemAlignSM<F>>,
    mem_align_rom_sm: Arc<MemAlignRomSM<F>>,
    input_data_sm: Arc<InputDataSM<F>>,
}

impl<F: PrimeField> MemProxy<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        let mem_align_rom_sm = MemAlignRomSM::new(wcm.clone());
        let mem_align_sm = MemAlignSM::new(wcm.clone(), std.clone(), mem_align_rom_sm.clone());
        let mem_sm = MemSM::new(wcm.clone(), std);
        let input_data_sm = InputDataSM::new(wcm.clone());

        let mem_proxy = Self {
            registered_predecessors: AtomicU32::new(0),
            mem_align_sm,
            mem_align_rom_sm,
            mem_sm,
            input_data_sm,
        };
        let mem_proxy = Arc::new(mem_proxy);

        wcm.register_component(mem_proxy.clone(), None, None);

        // For all the secondary state machines, register the main state machine as a predecessor
        mem_proxy.mem_align_rom_sm.register_predecessor();
        mem_proxy.mem_align_sm.register_predecessor();
        mem_proxy.mem_sm.register_predecessor();
        mem_proxy.input_data_sm.register_predecessor();
        mem_proxy
    }
    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.mem_align_rom_sm.unregister_predecessor();
            self.mem_align_sm.unregister_predecessor();
            self.mem_sm.unregister_predecessor();
            self.input_data_sm.unregister_predecessor();
        }
    }

    pub fn prove(
        &self,
        mem_operations: &mut Vec<ZiskRequiredMemory>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut engine = MemProxyEngine::<F>::new();
        engine.add_module("mem", self.mem_sm.clone());
        engine.add_module("input_data", self.input_data_sm.clone());
        engine.prove(&self.mem_align_sm, mem_operations)
    }
}

impl<F: PrimeField> WitnessComponent<F> for MemProxy<F> {}
