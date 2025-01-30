use data_bus::BusDevice;

pub struct BusDeviceWrapper<D> {
    instance_idx: Option<usize>,
    bus_device: Option<Box<dyn BusDevice<D>>>,
}

impl<D> BusDeviceWrapper<D> {
    pub fn new(instance_idx: Option<usize>, bus_device: Box<dyn BusDevice<D>>) -> Self {
        Self { instance_idx, bus_device: Some(bus_device) }
    }

    pub fn instance_idx(&self) -> Option<usize> {
        self.instance_idx
    }

    pub fn detach_device(&mut self) -> Box<dyn BusDevice<D>> {
        self.bus_device.take().expect("BusDeviceWrapper: No device to detach")
    }
}

impl<D: 'static> BusDevice<D> for BusDeviceWrapper<D> {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &data_bus::BusId,
        data: &[D],
    ) -> (bool, Vec<(data_bus::BusId, Vec<D>)>) {
        self.bus_device.as_mut().unwrap().process_data(bus_id, data)
    }

    fn bus_id(&self) -> Vec<data_bus::BusId> {
        self.bus_device.as_ref().unwrap().bus_id()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
