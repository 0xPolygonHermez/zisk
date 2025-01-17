//! The `BusDeviceMetrics` and `BusDeviceMetricsWrapper` modules integrate the functionalities
//! of `BusDevice` and `Metrics`, providing a unified interface for monitoring and managing
//! bus operations with associated metrics.

use data_bus::{BusDevice, BusId, PayloadType};

use crate::Metrics;

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
}

impl BusDeviceMetricsWrapper {
    /// Creates a new `BusDeviceMetricsWrapper` with the given inner `BusDeviceMetrics`.
    ///
    /// # Arguments
    /// * `inner` - A boxed implementation of the `BusDeviceMetrics` trait.
    ///
    /// # Returns
    /// A new `BusDeviceMetricsWrapper` instance.
    pub fn new(inner: Box<dyn BusDeviceMetrics>) -> Self {
        Self { inner }
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
    /// A tuple where:
    /// - The first element is a boolean indicating whether processing should continue.
    /// - The second element is a vector of tuples containing bus IDs and their associated data
    ///   payloads.
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> (bool, Vec<(BusId, Vec<u64>)>) {
        self.inner.process_data(bus_id, data)
    }
}
