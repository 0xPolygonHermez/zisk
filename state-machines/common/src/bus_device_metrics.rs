use zisk_common::{BusDevice, BusId, PayloadType};

use crate::Metrics;

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
