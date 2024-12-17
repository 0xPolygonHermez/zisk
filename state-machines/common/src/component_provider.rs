use p3_field::PrimeField;
use zisk_common::{BusDevice, BusId, PayloadType};

use crate::{Instance, InstanceExpanderCtx, Metrics, Planner};

pub trait ComponentProvider<F: PrimeField>: Send + Sync {
    fn get_counter(&self) -> Box<dyn BusDeviceWithMetrics>;
    fn get_planner(&self) -> Box<dyn Planner>;
    fn get_instance(&self, iectx: InstanceExpanderCtx) -> Box<dyn Instance<F>>;
}

pub trait BusDeviceWithMetrics: BusDevice<u64> + Metrics + std::any::Any {}

impl<T: BusDevice<u64> + Metrics + std::any::Any> BusDeviceWithMetrics for T {}

pub struct BusDeviceWrapper {
    pub inner: Box<dyn BusDeviceWithMetrics>,
}

impl BusDeviceWrapper {
    pub fn new(inner: Box<dyn BusDeviceWithMetrics>) -> Self {
        Self { inner }
    }
}

impl BusDevice<u64> for BusDeviceWrapper {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> Vec<(BusId, Vec<PayloadType>)> {
        self.inner.process_data(bus_id, data)
    }
}
