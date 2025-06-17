use crate::NestedDeviceMetricsList;
use data_bus::{DataBus, DataBusTrait};
use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::collections::HashMap;
use zisk_common::{BusDevice, BusDeviceMetrics, Instance, InstanceCtx, PayloadType, Plan};

pub type DataBusCollectorCollection = Vec<Option<DataBus<u64, Box<dyn BusDevice<u64>>>>>;
pub trait SMBundle<F: PrimeField64>: Send + Sync {
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
    fn build_data_bus_counters(
        &self,
    ) -> impl DataBusTrait<PayloadType, Box<dyn BusDeviceMetrics>> + Send + Sync + 'static;

    fn main_counter_idx(&self) -> Option<usize>;

    /// Retrieves a data bus for managing collectors in secondary state machines.
    /// # Arguments
    /// * `secn_instance` - Secondary state machine instance.
    /// * `chunks_to_execute` - A vector of booleans indicating which chunks to execute.
    ///
    /// # Returns
    /// A vector of data buses with attached collectors for each chunk to be executed
    #[allow(clippy::borrowed_box)]
    fn build_data_bus_collectors(
        &self,
        secn_instances: &HashMap<usize, &Box<dyn Instance<F>>>,
        chunks_to_execute: Vec<Vec<usize>>,
    ) -> DataBusCollectorCollection;
}
