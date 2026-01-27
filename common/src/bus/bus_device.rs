use std::any::Any;

/// Represents a subscriber in the `DataBus` system.
///
/// A `BusDevice` listens to messages sent to specific or all bus IDs and processes the data
/// accordingly.
///
/// # Associated Type
/// * `D` - The type of data handled by the `BusDevice`.
pub trait BusDevice<D>: Any + Send + Sync {
    /// Converts the device to a generic `Any` type.
    fn as_any(self: Box<Self>) -> Box<dyn Any>;
}
