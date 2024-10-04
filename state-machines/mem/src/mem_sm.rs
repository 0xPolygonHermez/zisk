use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::ZiskRequiredMemory;
use zisk_pil::{MEM_AIRGROUP_ID, MEM_ALIGN_AIR_IDS};

use crate::MemWitness;

pub struct MemSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredMemory>>,
}

impl<F: Field> MemSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_aligned_sm = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            inputs: Mutex::new(Vec::new()),
        };
        let mem_aligned_sm = Arc::new(mem_aligned_sm);

        wcm.register_component(
            mem_aligned_sm.clone(),
            Some(MEM_AIRGROUP_ID),
            Some(MEM_ALIGN_AIR_IDS),
        );

        mem_aligned_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            let mut mem_ops = self.inputs.lock().unwrap();

            MemWitness::prove_witnesses(
                std::mem::take(&mut *mem_ops),
                self.wcm.get_arc_pctx(),
                self.wcm.get_arc_ectx(),
                self.wcm.get_arc_sctx(),
                scope,
            )
            .expect("Failed to prove witnesses");
        }
    }
}

impl<F: Field> WitnessComponent<F> for MemSM<F> {}

impl<F: Field> Provable<ZiskRequiredMemory, OpResult> for MemSM<F> {
    fn prove(&self, operations: &[ZiskRequiredMemory], _drain: bool, _scope: &Scope) {
        self.inputs.lock().expect("Failed to lock inputs").extend_from_slice(operations);
    }
}
