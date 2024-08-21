use std::{collections::HashMap, sync::Arc};

use log::info;

use proofman_common::{ExecutionCtx, ProofCtx};
use crate::WitnessComponent;

pub struct WitnessManager<F> {
    components: Vec<Arc<dyn WitnessComponent<F>>>,
    airs: HashMap<usize, Arc<dyn WitnessComponent<F>>>,
}

impl<F> Default for WitnessManager<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> WitnessManager<F> {
    const MY_NAME: &'static str = "WCMnager";

    pub fn new() -> Self {
        WitnessManager { components: Vec::new(), airs: HashMap::new() }
    }

    pub fn register_component(&mut self, component: Arc<dyn WitnessComponent<F>>, air_ids: Option<&[usize]>) {
        if let Some(air_ids) = air_ids {
            self.register_airs(air_ids, component.clone());
        }

        self.components.push(component);
    }

    pub fn register_airs(&mut self, air_ids: &[usize], air: Arc<dyn WitnessComponent<F>>) {
        for air_id in air_ids.iter() {
            self.register_air(*air_id, air.clone());
        }
    }

    pub fn register_air(&mut self, air_id: usize, air: Arc<dyn WitnessComponent<F>>) {
        if self.airs.contains_key(&air_id) {
            panic!("{}: Air ID {} already registered", Self::MY_NAME, air_id);
        }

        self.airs.insert(air_id, air);
    }

    pub fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        for component in self.components.iter() {
            component.start_proof(pctx, ectx);
        }
    }

    pub fn end_proof(&mut self) {
        for component in self.components.iter() {
            component.end_proof();
        }
    }

    pub fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        info!("{}: Calculating witness (stage {})", Self::MY_NAME, stage);

        let air_instances = pctx.air_instances.read().unwrap();

        let mut components = HashMap::new();
        for (air_instance_id, air_instance_ctx) in air_instances.iter().enumerate() {
            let component = self.airs.get(&air_instance_ctx.air_id).unwrap();

            // Use the `entry` API to efficiently insert or push to the Vec
            components.entry(air_instance_ctx.air_id).or_insert_with(Vec::new).push((component, air_instance_id));
        }

        drop(air_instances);

        for component_group in components.values() {
            for (component, id) in component_group.iter() {
                component.calculate_witness(stage, *id, pctx, ectx);
            }
        }
    }
}
