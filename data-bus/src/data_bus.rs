//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.

use std::{any::Any, collections::HashMap};

/// Type representing the unique identifier of a bus channel.
pub type BusId = u16;

/// Type representing the payload transmitted across the bus.
pub type PayloadType = u64;

/// Type representing a memory data payload consisting of four `PayloadType` values.
pub type MemData = [PayloadType; 4];

/// Represents a subscriber in the `DataBus` system.
///
/// A `BusDevice` listens to messages sent to specific or all bus IDs and processes the data
/// accordingly.
///
/// # Associated Type
/// * `D` - The type of data handled by the `BusDevice`.
pub trait BusDevice<D>: Any + Send {
    /// Processes incoming data sent to the device.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus that sent the data.
    /// * `data` - A reference to the data payload being processed.
    ///
    /// # Returns
    /// A tuple containing:
    /// - `bool` indicating whether processing should end.
    /// - A vector of `(BusId, Vec<D>)` representing additional data to be sent to other bus IDs.
    fn process_data(&mut self, bus_id: &BusId, data: &[D]) -> Option<Vec<(BusId, Vec<D>)>> {
        let _ = bus_id;
        let _ = data;
        None
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId>;

    fn as_any(self: Box<Self>) -> Box<dyn Any>;
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
    pub devices: Vec<Box<BD>>,

    /// Mapping from `BusId` to indices of devices listening to that ID.
    devices_bus_id_map: HashMap<BusId, Vec<usize>>,

    /// List of global (omni) devices that process all data.
    omni_devices: Vec<usize>,

    /// Queue of pending data transfers to be processed.
    pending_transfers: Vec<(BusId, Vec<D>)>,
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
            devices_bus_id_map: HashMap::new(),
            omni_devices: Vec::new(),
            pending_transfers: Vec::new(),
        }
    }

    /// Connects a device to the bus with specific `BusId` subscriptions.
    ///
    /// # Arguments
    /// * `bus_ids` - A vector of `BusId` values the device subscribes to.
    /// * `bus_device` - The device to be added to the bus.
    pub fn connect_device(&mut self, bus_ids: Vec<BusId>, bus_device: Box<BD>) {
        self.devices.push(bus_device);
        let device_idx = self.devices.len() - 1;

        for bus_id in bus_ids {
            self.devices_bus_id_map.entry(bus_id).or_default().push(device_idx);
        }
    }

    /// Connects a global (omni) device that processes all bus data.
    ///
    /// # Arguments
    /// * `bus_device` - The global device to be added.
    pub fn connect_omni_device(&mut self, bus_device: Box<BD>) {
        self.devices.push(bus_device);
        let device_idx = self.devices.len() - 1;

        self.omni_devices.push(device_idx);
    }

    /// Writes data to the bus and processes it through the registered devices.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus receiving the data.
    /// * `payload` - The data payload to be sent.
    ///
    /// # Returns
    /// `true` if processing completed successfully, otherwise `false`.
    pub fn write_to_bus(&mut self, bus_id: BusId, payload: Vec<D>) {
        self.pending_transfers.push((bus_id, payload));

        while let Some((bus_id, payload)) = self.pending_transfers.pop() {
            self.route_data(bus_id, &payload)
        }
    }

    /// Routes data to the devices subscribed to a specific bus ID or global devices.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to route the data to.
    /// * `payload` - A reference to the data payload being routed.
    ///
    /// # Returns
    /// `true` if processing completed successfully, otherwise `false`.
    fn route_data(&mut self, bus_id: BusId, payload: &[D]) {
        // Notify specific subscribers
        if let Some(bus_id_devices) = self.devices_bus_id_map.get(&bus_id) {
            for &device_idx in bus_id_devices {
                if let Some(result) = self.devices[device_idx].process_data(&bus_id, payload) {
                    self.pending_transfers.extend(result);
                }
            }
        }

        // Notify global (omni) subscribers
        for &device_idx in &self.omni_devices {
            if let Some(result) = self.devices[device_idx].process_data(&bus_id, payload) {
                self.pending_transfers.extend(result);
            }
        }
    }

    /// Outputs the current state of the bus for debugging purposes.
    pub fn debug_state(&self) {
        println!("Devices: {:?}", self.devices.len());
        println!("Devices by bus ID: {:?}", self.devices_bus_id_map);
        println!("Global Devices: {:?}", self.omni_devices.len());
        println!("Pending Transfers: {:?}", self.pending_transfers.len());
    }

    /// Detaches and returns the most recently added device.
    ///
    /// # Returns
    /// An optional `Box<BD>` representing the detached device, or `None` if no devices are
    /// connected.
    pub fn detach_first_device(&mut self) -> Option<Box<BD>> {
        self.devices.pop()
    }

    /// Detaches and returns all devices currently connected to the bus.
    ///
    /// # Returns
    /// A vector of `Box<BD>` representing all detached devices.
    pub fn detach_devices(&mut self) -> Vec<Box<BD>> {
        std::mem::take(&mut self.devices)
    }
}
