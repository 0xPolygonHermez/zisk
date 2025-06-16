use std::sync::Arc;

use data_bus::{DataBus, DataBusTrait};
use fields::PrimeField64;
use proofman_common::ProofCtx;
use sm_main::MainSM;
use zisk_common::{
    BusDevice, BusDeviceMetrics, ChunkId, ComponentBuilder, Instance, InstanceCtx, PayloadType,
    Plan,
};

use crate::{NestedDeviceMetricsList, SMBundle};
use std::collections::HashMap;

pub struct DynSMBundle<F: PrimeField64> {
    secondary_sm: Vec<Arc<dyn ComponentBuilder<F>>>,
}

impl<F: PrimeField64> DynSMBundle<F> {
    pub fn new(secondary_sm: Vec<Arc<dyn ComponentBuilder<F>>>) -> Self {
        Self { secondary_sm }
    }
}

impl<F: PrimeField64> SMBundle<F> for DynSMBundle<F> {
    fn plan_sec(&self, vec_counters: NestedDeviceMetricsList) -> Vec<Vec<Plan>> {
        self.secondary_sm
            .iter()
            .zip(vec_counters)
            .map(|(sm, counters)| sm.build_planner().plan(counters))
            .collect()
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Vec<Plan>]) {
        self.secondary_sm
            .iter()
            .zip(plannings)
            .for_each(|(sm, plans)| sm.configure_instances(pctx, plans));
    }

    fn build_instance(&self, idx: usize, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        self.secondary_sm[idx].build_instance(ictx)
    }

    fn build_data_bus_counters(
        &self,
    ) -> impl DataBusTrait<PayloadType, Box<dyn BusDeviceMetrics>> + Send + Sync + 'static {
        let mut data_bus = DataBus::new();

        let counter = MainSM::build_counter();

        data_bus.connect_device(None, Some(counter));

        self.secondary_sm.iter().for_each(|sm| {
            let counter = sm.build_counter();

            data_bus.connect_device(None, counter);
        });

        data_bus
    }

    fn main_counter_idx(&self) -> Option<usize> {
        Some(0)
    }

    fn build_data_bus_collectors(
        &self,
        secn_instances: &HashMap<usize, &Box<dyn Instance<F>>>,
        chunks_to_execute: Vec<Vec<usize>>,
    ) -> Vec<Option<DataBus<u64, Box<dyn BusDevice<u64>>>>> {
        chunks_to_execute
            .iter()
            .enumerate()
            .map(|(chunk_id, global_idxs)| {
                if global_idxs.is_empty() {
                    return None;
                }

                let mut data_bus = DataBus::new();

                let mut used = false;
                for global_idx in global_idxs {
                    let secn_instance = secn_instances.get(global_idx).unwrap();
                    if let Some(bus_device) =
                        secn_instance.build_inputs_collector(ChunkId(chunk_id))
                    {
                        data_bus.connect_device(Some(*global_idx), Some(bus_device));

                        used = true;
                    }
                }

                if used {
                    for sm in &self.secondary_sm {
                        if let Some(inputs_generator) = sm.build_inputs_generator() {
                            data_bus.connect_device(None, Some(inputs_generator));
                        }
                    }
                    Some(data_bus)
                } else {
                    None
                }
            })
            .collect()
    }
}
