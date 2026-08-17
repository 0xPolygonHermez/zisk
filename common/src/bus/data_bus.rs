//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.

use crate::BusId;

/// The `DataBusTrait` defines the interface for a data bus that allows writing data to the bus and
/// processing it through registered devices.
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
    fn write_to_bus(&mut self, bus_id: BusId, data: &[D], data_ext: &[D]) -> bool;

    /// Called when the bus is closed, allowing for cleanup or finalization of resources.
    fn on_close(&mut self);

    /// Converts the data bus into a vector of devices, optionally executing the `on_close` method
    /// for each device.
    ///
    /// # Arguments
    /// * `execute_on_close` - If `true`, the `on_close` method will be called for each device.
    ///
    /// # Returns
    /// A vector of tuples containing the global idx and the corresponding device instance.
    fn into_devices(self, execute_on_close: bool) -> Vec<(usize, T)>;
}
