use std::sync::Arc;

use data_bus::{DataBus, DataBusTrait};
use p3_field::{Field, PrimeField64};
use proofman_common::ProofCtx;
use sm_main::MainSM;
use zisk_common::{
    BusDevice, BusDeviceMetricsWrapper, BusDeviceWrapper, ChunkId, ComponentBuilder, Instance,
    InstanceCtx, PayloadType, Plan, OPERATION_BUS_ID,
};

use crate::NestedDeviceMetricsList;

pub trait SMBundle<F: Field>: Send + Sync {
    /// Plans the secondary state machines by generating plans from the counted metrics.
    ///
    /// # Arguments
    /// * `vec_counters` - A nested vector containing metrics for each secondary state machine.
    ///
    /// # Returns
    /// A vector of plans for each secondary state machine.
    fn plan_sec(&self, vec_counters: NestedDeviceMetricsList) -> Vec<Vec<Plan>>;

    /// Prepares and configures the secondary instances using the provided plans before their
    /// creation.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `plannings` - A vector of vectors containing plans for each secondary state machine.
    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Vec<Plan>]);

    fn build_instance(&self, idx: usize, ictx: InstanceCtx) -> Box<dyn Instance<F>>;

    /// Retrieves a `DataBus` configured with counters for each secondary state machine.
    ///
    /// # Returns
    /// A `DataBus` instance with connected counters for each registered secondary state machine.
    fn get_data_bus_counters(&self) -> impl DataBusTrait<PayloadType> + Send + Sync + 'static;

    /// Retrieves a data bus for managing collectors in secondary state machines.
    /// # Arguments
    /// * `secn_instance` - Secondary state machine instance.
    /// * `chunks_to_execute` - A vector of booleans indicating which chunks to execute.
    ///
    /// # Returns
    /// A vector of data buses with attached collectors for each chunk to be executed
    fn get_data_bus_collectors(
        &self,
        secn_instance: &mut Box<dyn Instance<F>>,
        chunks_to_execute: Vec<bool>,
    ) -> Vec<Option<DataBus<u64, BusDeviceWrapper<u64>>>>;
}

pub struct DynSMBundle<F: PrimeField64> {
    // main_sm: Arc<dyn ComponentBuilder<F>>,
    secondary_sm: Vec<Arc<dyn ComponentBuilder<F>>>,
}

impl<F: PrimeField64> DynSMBundle<F> {
    pub fn new(
        // main_sm: Arc<dyn ComponentBuilder<F>>,
        secondary_sm: Vec<Arc<dyn ComponentBuilder<F>>>,
    ) -> Self {
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

    fn get_data_bus_counters(&self) -> impl DataBusTrait<PayloadType> + Send + Sync + 'static {
        let mut data_bus = DataBus::new();

        let counter = MainSM::build_counter();

        data_bus.connect_device(counter.bus_id(), BusDeviceMetricsWrapper::new(counter, false));

        self.secondary_sm.iter().for_each(|sm| {
            let counter = sm.build_counter();

            data_bus.connect_device(counter.bus_id(), BusDeviceMetricsWrapper::new(counter, true));
        });

        data_bus
    }

    fn get_data_bus_collectors(
        &self,
        secn_instance: &mut Box<dyn Instance<F>>,
        chunks_to_execute: Vec<bool>,
    ) -> Vec<Option<DataBus<u64, BusDeviceWrapper<u64>>>> {
        chunks_to_execute
            .iter()
            .enumerate()
            .map(|(chunk_id, to_be_executed)| {
                if !to_be_executed {
                    return None;
                }

                let mut data_bus = DataBus::new();

                if let Some(bus_device) = secn_instance.build_inputs_collector(ChunkId(chunk_id)) {
                    let bus_device = BusDeviceWrapper::new(bus_device);
                    data_bus.connect_device(bus_device.bus_id(), bus_device);

                    for sm in &self.secondary_sm {
                        if let Some(inputs_generator) = sm.build_inputs_generator() {
                            data_bus.connect_device(
                                vec![OPERATION_BUS_ID],
                                BusDeviceWrapper::new(inputs_generator),
                            );
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
