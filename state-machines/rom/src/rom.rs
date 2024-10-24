use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::SetupCtx;
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::ZiskRequiredOperation;

pub struct RomSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,
}

impl RomSM {
    pub fn new<F>(wcm: Arc<WitnessManager<F>>, sctx: Arc<SetupCtx>) -> Arc<Self> {
        let rom_sm = Self { registered_predecessors: AtomicU32::new(0) };
        let rom_sm = Arc::new(rom_sm);

        // FIXME! Remove following constants and replace with pilout values when available
        const ROM_AIRGROUP_ID: usize = 110;
        const ROM_AIR_IDS: &[usize] = &[0];

        wcm.register_component(rom_sm.clone(), Some(ROM_AIRGROUP_ID), Some(ROM_AIR_IDS));

        rom_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor<F: Field>(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <RomSM as Provable<ZiskRequiredOperation, OpResult>>::prove(self, &[], true, scope);
        }
    }
}

impl<F> WitnessComponent<F> for RomSM {}

impl Provable<ZiskRequiredOperation, OpResult> for RomSM {
    fn prove(&self, _operations: &[ZiskRequiredOperation], _drain: bool, _scope: &Scope) {}
}
