use p3_field::PrimeField;
use zisk_common::{BusDevice, BusId, PayloadType};

use crate::Instance;

pub trait BusDeviceInstance<F: PrimeField>: BusDevice<u64> + Instance<F> + std::any::Any {}

impl<F: PrimeField, T: BusDevice<u64> + Instance<F> + std::any::Any> BusDeviceInstance<F> for T {}

/// Shared wrapper to encapsulate dual functionality (BusDevice + Instance) in a single object.
pub struct BusDeviceInstanceWrapper<F: PrimeField> {
    pub inner: Box<dyn BusDeviceInstance<F>>,
}

impl<F: PrimeField> BusDeviceInstanceWrapper<F> {
    pub fn new(inner: Box<dyn BusDeviceInstance<F>>) -> Self {
        Self { inner }
    }
}

impl<F: PrimeField> BusDevice<u64> for BusDeviceInstanceWrapper<F> {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> (bool, Vec<(BusId, Vec<u64>)>) {
        self.inner.process_data(bus_id, data)
    }
}
