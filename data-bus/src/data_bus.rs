//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.

use std::collections::VecDeque;

use zisk_common::{BusDevice, BusId};

pub trait DataBusTrait<D, T> {
    /// Writes data to the bus and processes it through the registered devices.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus receiving the data.
    /// * `payload` - The data payload to be sent.
    fn write_to_bus(&mut self, bus_id: BusId, payload: &[D]);

    fn on_close(&mut self);

    fn into_devices(self, execute_on_close: bool) -> Vec<T>;
}

/// A bus system facilitating communication between multiple publishers and subscribers.
///
/// The `DataBus` allows devices to register for specific bus IDs or act as global (omni) devices.
/// It routes payloads to registered devices and handles data transfers efficiently.
///
/// # Type Parameters
/// * `D` - The type of data payloads handled by the bus.
/// * `BD` - The type of devices (subscribers) connected to the bus, implementing the `BusDevice`
///   trait.
pub struct DataBus<D, BD: BusDevice<D>> {
    /// List of devices connected to the bus.
    pub devices: Vec<BD>,

    /// Mapping from `BusId` to indices of devices listening to that ID.
    devices_bus_id_map: Vec<Vec<usize>>,

    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>)>,
}

impl<D, BD: BusDevice<D>> Default for DataBus<D, BD> {
    /// Creates a new `DataBus` with default settings.
    fn default() -> Self {
        Self::new()
    }
}

impl<D, BD: BusDevice<D>> DataBus<D, BD> {
    /// Creates a new `DataBus` instance.
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            devices_bus_id_map: vec![vec![], vec![], vec![]],
            pending_transfers: VecDeque::new(),
        }
    }

    /// Connects a device to the bus with specific `BusId` subscriptions.
    ///
    /// # Arguments
    /// * `bus_ids` - A vector of `BusId` values the device subscribes to.
    /// * `bus_device` - The device to be added to the bus.
    pub fn connect_device(&mut self, bus_ids: Vec<BusId>, bus_device: BD) {
        self.devices.push(bus_device);
        let device_idx = self.devices.len() - 1;

        for bus_id in bus_ids {
            self.devices_bus_id_map[*bus_id].push(device_idx);
        }
    }

    /// Routes data to the devices subscribed to a specific bus ID or global devices.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to route the data to.
    /// * `payload` - A reference to the data payload being routed.
    #[inline(always)]
    fn route_data(&mut self, bus_id: BusId, payload: &[D]) {
        // Notify specific subscribers
        let bus_id_devices = &self.devices_bus_id_map[*bus_id];
        for device_idx in bus_id_devices {
            if let Some(result) = self.devices[*device_idx].process_data(&bus_id, payload) {
                self.pending_transfers.extend(result);
            }
        }
    }

    /// Outputs the current state of the bus for debugging purposes.
    pub fn debug_state(&self) {
        println!("Devices: {:?}", self.devices.len());
        println!("Devices by bus ID: {:?}", self.devices_bus_id_map);
        println!("Pending Transfers: {:?}", self.pending_transfers.len());
    }
}

impl<D, BD: BusDevice<D>> DataBusTrait<D, BD> for DataBus<D, BD> {
    fn write_to_bus(&mut self, bus_id: BusId, payload: &[D]) {
        self.route_data(bus_id, payload);

        while let Some((bus_id, payload)) = self.pending_transfers.pop_front() {
            self.route_data(bus_id, &payload)
        }
    }

    fn on_close(&mut self) {
        for device in &mut self.devices {
            device.on_close();
        }
    }

    fn into_devices(self, execute_on_close: bool) -> Vec<BD> {
        let mut result = Vec::with_capacity(self.devices.len());

        for mut device in self.devices {
            if execute_on_close {
                device.on_close();
            }
            result.push(device);
        }

        result
    }
}
