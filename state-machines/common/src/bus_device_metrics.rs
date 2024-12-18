use zisk_common::{BusDevice, BusId, PayloadType};

use crate::Metrics;

pub trait BusDeviceMetrics: BusDevice<u64> + Metrics + std::any::Any {}

impl<T: BusDevice<u64> + Metrics + std::any::Any> BusDeviceMetrics for T {}

pub struct BusDeviceMetricsWrapper {
    pub inner: Box<dyn BusDeviceMetrics>,
}

impl BusDeviceMetricsWrapper {
    pub fn new(inner: Box<dyn BusDeviceMetrics>) -> Self {
        Self { inner }
    }
}

impl BusDevice<u64> for BusDeviceMetricsWrapper {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> Vec<(BusId, Vec<PayloadType>)> {
        self.inner.process_data(bus_id, data)
    }
}
