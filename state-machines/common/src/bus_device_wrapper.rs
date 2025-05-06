use zisk_common::{BusDevice, BusId};

/// The `BusDeviceWrapper` struct wraps a `BusDevice` trait object
pub struct BusDeviceWrapper<D> {
    /// The wrapped `BusDevice` trait object.
    bus_device: Option<Box<dyn BusDevice<D>>>,
}

impl<D> BusDeviceWrapper<D> {
    /// Creates a new `BusDeviceWrapper` instance.
    ///
    /// # Arguments
    /// instance_idx - The instance index of the device.
    /// bus_device - The boxed `BusDevice` trait object.
    ///
    /// # Returns
    /// A new `BusDeviceWrapper` instance.
    pub fn new(bus_device: Box<dyn BusDevice<D>>) -> Self {
        Self { bus_device: Some(bus_device) }
    }

    /// Detaches the device from the wrapper.
    ///
    /// # Returns
    /// The detached `BusDevice` trait object.
    ///
    /// # Panics
    /// If there is no device to detach.
    pub fn detach_device(&mut self) -> Box<dyn BusDevice<D>> {
        self.bus_device.take().expect("BusDeviceWrapper: No device to detach")
    }
}

impl<D: 'static> BusDevice<D> for BusDeviceWrapper<D> {
    /// Processes incoming data sent to the device.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus that sent the data.
    /// * `data` - A reference to the data payload being processed.
    ///
    /// # Returns
    /// An optional vector of tuples containing the bus ID and data payload to be sent to other
    /// devices. If no data is to be sent, `None` is returned.
    #[inline(always)]
    fn process_data(&mut self, bus_id: &BusId, data: &[D]) -> Option<Vec<(BusId, Vec<D>)>> {
        self.bus_device.as_mut().unwrap().process_data(bus_id, data)
    }

    /// Returns the bus IDs associated with this instance.
    fn bus_id(&self) -> Vec<BusId> {
        self.bus_device.as_ref().unwrap().bus_id()
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
