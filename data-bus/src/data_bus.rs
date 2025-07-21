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
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    fn write_to_bus(&mut self, bus_id: BusId, payload: &[D]) -> bool;

    fn on_close(&mut self);

    fn into_devices(self, execute_on_close: bool) -> Vec<(Option<usize>, Option<T>)>;
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
    devices: Vec<(Option<usize>, BD)>,

    /// Mapping from `BusId` to indices of devices listening to that ID.
    devices_bus_id_map: Vec<Vec<usize>>,

    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>)>,

    /// Indices of devices that are connected to the bus but without a specific instance.
    none_devices: Vec<usize>,

    /// The number of active devices currently connected to the bus.
    active_devices: usize,
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
            none_devices: vec![],
            active_devices: 0,
        }
    }

    /// Connects a device to the bus with specific `BusId` subscriptions.
    ///
    /// # Arguments
    /// * `instance_idx` - An optional index for the device instance.
    /// * `bus_device` - The device to be added to the bus.
    pub fn connect_device(&mut self, instance_idx: Option<usize>, bus_device: Option<BD>) {
        if let Some(bus_device) = bus_device {
            let bus_ids = bus_device.bus_id();

            self.devices.push((instance_idx, bus_device));
            let device_idx = self.devices.len() - 1;

            for bus_id in bus_ids {
                self.devices_bus_id_map[*bus_id].push(device_idx);
            }
            self.active_devices += 1;
        } else {
            self.none_devices.push(self.devices.len());
        }
    }

    /// Routes data to the devices subscribed to a specific bus ID or global devices.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to route the data to.
    /// * `payload` - A reference to the data payload being routed.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    fn route_data(&mut self, bus_id: BusId, payload: &[D]) {
        let devices_idx = &mut self.devices_bus_id_map[*bus_id];
        let mut i = 0;

        while i < devices_idx.len() {
            let device_idx = devices_idx[i];
            // When a device returns false, it indicates that it has finished its work and should be disabled.
            if !self.devices[device_idx].1.process_data(
                &bus_id,
                payload,
                &mut self.pending_transfers,
            ) {
                // Remove the device from the bus and update the mapping.
                devices_idx.swap_remove(i);
                self.active_devices -= 1;
            } else {
                i += 1;
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
    /// Writes data to the bus and processes it through the registered devices.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus receiving the data.
    /// * `payload` - The data payload to be sent.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    fn write_to_bus(&mut self, bus_id: BusId, payload: &[D]) -> bool {
        self.route_data(bus_id, payload);

        while let Some((bus_id, payload)) = self.pending_transfers.pop_front() {
            self.route_data(bus_id, &payload);
        }

        self.active_devices > 0
    }

    /// Called when the bus is closed, allowing devices to perform any necessary cleanup.
    fn on_close(&mut self) {
        for device in &mut self.devices {
            device.1.on_close();
        }
    }

    /// Converts the bus into a vector of devices, optionally executing their close operations.
    ///
    /// # Arguments
    /// * `execute_on_close` - If true, calls the `on_close` method on each device.
    ///
    //// # Returns
    /// A vector of tuples containing the device instance index and the device itself.
    fn into_devices(self, execute_on_close: bool) -> Vec<(Option<usize>, Option<BD>)> {
        let total_len = self.devices.len() + self.none_devices.len();
        let mut result = Vec::with_capacity(total_len);

        let mut dev_iter = self.devices.into_iter();
        let mut none_iter = self.none_devices.iter().copied().peekable();

        for idx in 0..total_len {
            if Some(&idx) == none_iter.peek() {
                result.push((None, None));
                none_iter.next();
            } else {
                let mut device =
                    dev_iter.next().expect("Mismatch between device and none-device count");

                if execute_on_close {
                    device.1.on_close();
                }

                result.push((device.0, Some(device.1)));
            }
        }

        result
    }
}
