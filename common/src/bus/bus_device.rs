use std::{any::Any, collections::VecDeque};

use super::BusId;

/// Represents a subscriber in the `DataBus` system.
///
/// A `BusDevice` listens to messages sent to specific or all bus IDs and processes the data
/// accordingly.
///
/// # Associated Type
/// * `D` - The type of data handled by the `BusDevice`.
pub trait BusDevice<D>: Any + Send + Sync {
    /// Processes incoming data sent to the device.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus that sent the data.
    /// * `data` - A reference to the data payload being processed.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[D],
        pending: &mut VecDeque<(BusId, Vec<D>)>,
    ) -> bool;

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId>;

    /// Converts the device to a generic `Any` type.
    fn as_any(self: Box<Self>) -> Box<dyn Any>;

    /// Performs any necessary cleanup or finalization when the metrics instance is closed.
    fn on_close(&mut self) {}
}

impl BusDevice<u64> for Box<dyn BusDevice<u64>> {
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        (**self).process_data(bus_id, data, pending)
    }

    fn bus_id(&self) -> Vec<BusId> {
        (**self).bus_id()
    }

    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        (*self).as_any()
    }

    fn on_close(&mut self) {
        (**self).on_close()
    }
}
