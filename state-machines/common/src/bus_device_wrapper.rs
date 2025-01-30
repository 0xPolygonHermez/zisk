use data_bus::BusDevice;

pub struct BusDeviceWrapper<D> {
    instance_idx: Option<usize>,
    bus_device: Box<dyn BusDevice<D>>,
}

impl<D> BusDeviceWrapper<D> {
    pub fn new(instance_idx: Option<usize>, bus_device: Box<dyn BusDevice<D>>) -> Self {
        Self { instance_idx, bus_device }
    }

    pub fn instance_idx(&self) -> Option<usize> {
        self.instance_idx
    }

    pub fn bus_device(&self) -> &dyn BusDevice<D> {
        &*self.bus_device
    }
}

impl<D: 'static> BusDevice<D> for BusDeviceWrapper<D> {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &data_bus::BusId,
        data: &[D],
    ) -> (bool, Vec<(data_bus::BusId, Vec<D>)>) {
        self.bus_device.process_data(bus_id, data)
    }

    fn bus_id(&self) -> Vec<data_bus::BusId> {
        self.bus_device.bus_id()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
