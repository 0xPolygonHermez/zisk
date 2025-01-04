use zisk_common::{BusDevice, BusId, PayloadType};

use crate::Metrics;

pub trait BusDeviceMetrics: BusDevice<u64> + Metrics + std::any::Any {}

impl<T: BusDevice<u64> + Metrics + std::any::Any> BusDeviceMetrics for T {}

/// Shared wrapper to encapsulate dual functionality (BusDevice + Metrics) in a single object.
pub struct BusDeviceMetricsWrapper {
    
    pub inner: Box<dyn BusDeviceMetrics>,
}

impl BusDeviceMetricsWrapper {
    pub fn new(inner: Box<dyn BusDeviceMetrics>) -> Self {
        Self { inner }
    }

    #[inline(always)]
    pub fn on_close(&mut self) {
        self.inner.on_close();
    }
}

impl BusDevice<u64> for BusDeviceMetricsWrapper {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> (bool, Vec<(BusId, Vec<u64>)>) {
        self.inner.process_data(bus_id, data)
    }
}
