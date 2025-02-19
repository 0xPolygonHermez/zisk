//! The `BusDeviceMetrics` and `BusDeviceMetricsWrapper` modules integrate the functionalities
//! of `BusDevice` and `Metrics`, providing a unified interface for monitoring and managing
//! bus operations with associated metrics.

use data_bus::{BusDevice, BusId, PayloadType};

use crate::Metrics;

#[derive(Debug, PartialEq)]
pub enum BusDeviceMode {
    Counter,
    InputGenerator,
}

/// The `BusDeviceMetrics` trait combines the functionalities of `BusDevice` and `Metrics`,
/// enabling components to act as both bus devices and metric collectors.
///
/// This trait is particularly useful for tracking and analyzing operations on the bus while
/// maintaining compatibility with `Metrics` functionality.
pub trait BusDeviceMetrics: BusDevice<u64> + Metrics + std::any::Any {}

/// Blanket implementation of `BusDeviceMetrics` for any type implementing `BusDevice`,
/// `Metrics`, and `std::any::Any`.
impl<T: BusDevice<u64> + Metrics + std::any::Any> BusDeviceMetrics for T {}

/// The `BusDeviceMetricsWrapper` struct encapsulates an object implementing the
/// `BusDeviceMetrics` trait, providing a unified interface to manage both bus operations
/// and associated metrics.
///
/// This wrapper is particularly useful when you need to manage metrics and bus operations
/// together within a single abstraction.
pub struct BusDeviceMetricsWrapper {
    /// The inner boxed `BusDeviceMetrics`.
    pub inner: Box<dyn BusDeviceMetrics>,

    /// A flag indicating whether the device is a secondary device.
    pub is_secondary: bool,
}

impl BusDeviceMetricsWrapper {
    /// Creates a new `BusDeviceMetricsWrapper` with the given inner `BusDeviceMetrics`.
    ///
    /// # Arguments
    /// * `inner` - A boxed implementation of the `BusDeviceMetrics` trait.
    /// * `is_secondary` - A flag indicating whether the device is a secondary device.
    ///
    /// # Returns
    /// A new `BusDeviceMetricsWrapper` instance.
    pub fn new(inner: Box<dyn BusDeviceMetrics>, is_secondary: bool) -> Self {
        Self { inner, is_secondary }
    }

    /// Invokes the `on_close` method of the inner `BusDeviceMetrics`.
    ///
    /// This method is intended to perform cleanup or finalization tasks
    /// when the device or metrics collector is being closed.
    #[inline(always)]
    pub fn on_close(&mut self) {
        self.inner.on_close();
    }
}

impl BusDevice<u64> for BusDeviceMetricsWrapper {
    /// Processes data received on the bus, delegating the processing to the inner
    /// `BusDeviceMetrics`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The payload data received from the bus.
    ///
    /// # Returns
    /// An optional vector of tuples where:
    /// - The first element is the bus ID.
    /// - The second element contains the derived inputs to be sent back to the bus.
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> Option<Vec<(BusId, Vec<u64>)>> {
        self.inner.process_data(bus_id, data)
    }

    /// Returns the bus IDs associated with the inner `BusDeviceInstance`.
    /// This method delegates the call to the inner `BusDeviceInstance`.
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        self.inner.bus_id()
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
