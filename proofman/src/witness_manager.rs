use std::{collections::HashMap, sync::Arc};

use log::info;

use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use crate::WitnessComponent;

pub struct WitnessManager<F> {
    components: Vec<Arc<dyn WitnessComponent<F>>>,
    airs: HashMap<usize, usize>, // First usize is the air_id, second usize is the index of the component in the components vector
}

impl<F> Default for WitnessManager<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> WitnessManager<F> {
    const MY_NAME: &'static str = "WCMnager";

    pub fn new() -> Self {
        WitnessManager {
            components: Vec::new(),
            airs: HashMap::new(),
        }
    }

    pub fn register_component(&mut self, component: Arc<dyn WitnessComponent<F>>, air_ids: Option<&[usize]>) {
        self.components.push(component);

        let idx = self.components.len() - 1;

        if let Some(air_ids) = air_ids {
            self.register_airs(air_ids, idx);
        }
    }

    pub fn register_airs(&mut self, air_ids: &[usize], component_idx: usize) {
        for air_id in air_ids.iter() {
            self.register_air(*air_id, component_idx);
        }
    }

    pub fn register_air(&mut self, air_id: usize, component_idx: usize) {
        if self.airs.contains_key(&air_id) {
            panic!("{}: Air ID {} already registered", Self::MY_NAME, air_id);
        }

        self.airs.insert(air_id, component_idx);
    }

    pub fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        log::info!("{}: --> Starting proof", Self::MY_NAME);

        for component in self.components.iter() {
            component.start_proof(pctx, ectx, sctx);
        }
    }

    pub fn end_proof(&mut self) {
        log::info!("{}: <-- Finalizing proof", Self::MY_NAME);

        for component in self.components.iter() {
            component.end_proof();
        }
    }

    pub fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        info!("{}: --> Calculating witness (stage {})", Self::MY_NAME, stage);

        let air_instances = pctx.air_instances.read().unwrap();

        let mut components = HashMap::new();
        for (air_instance_id, air_instance_ctx) in air_instances.iter().enumerate() {
            let component = self.airs.get(&air_instance_ctx.air_id).unwrap();

            components.entry(air_instance_ctx.air_id).or_insert_with(Vec::new).push((component, air_instance_id));
        }

        drop(air_instances);

        // Call all used components
        let mut used_components = Vec::new();
        for component_group in components.values() {
            for (component_idx, id) in component_group.iter() {
                let component = &self.components[**component_idx];
                component.calculate_witness(stage, Some(*id), pctx, ectx, sctx);
                used_components.push(**component_idx);
            }
        }

        // Call one time all unused components
        for component_idx in 0..self.components.len() {
            if !used_components.contains(&component_idx) {
                self.components[component_idx].calculate_witness(stage, None, pctx, ectx, sctx);
            }
        }

        info!("{}: <-- Calculated witness (stage {})", Self::MY_NAME, stage);
    }
}
