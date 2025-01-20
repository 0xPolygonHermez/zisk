//! The `BusDeviceInstance` and `BusDeviceInstanceWrapper` modules integrate the dual functionality
//! of `BusDevice` and `Instance` traits, enabling seamless handling of bus operations and
//! instance management within a unified interface.
//!
//! These abstractions are particularly useful for scenarios that require encapsulating the
//! functionalities of a bus device and an instance while maintaining type safety and extensibility.

use data_bus::{BusDevice, BusId, PayloadType};
use p3_field::PrimeField;

use crate::Instance;

/// The `BusDeviceInstance` trait extends both `BusDevice` and `Instance` traits,
/// combining their functionalities into a single cohesive interface.
///
/// # Type Parameters
/// * `F` - A type implementing the `PrimeField` trait, representing the field over which operations
///   are performed.
pub trait BusDeviceInstance<F: PrimeField>: BusDevice<u64> + Instance<F> + std::any::Any {}

/// Blanket implementation of `BusDeviceInstance` for any type implementing `BusDevice`,
/// `Instance`, and `std::any::Any`.
impl<F: PrimeField, T: BusDevice<u64> + Instance<F> + std::any::Any> BusDeviceInstance<F> for T {}

/// The `BusDeviceInstanceWrapper` struct provides a shared wrapper to encapsulate an object
/// implementing the `BusDeviceInstance` trait.
///
/// This wrapper is useful for managing components that need to interact with both `BusDevice`
/// and `Instance` functionalities in a unified manner.
///
/// # Type Parameters
/// * `F` - A type implementing the `PrimeField` trait.
pub struct BusDeviceInstanceWrapper<F: PrimeField> {
    /// The inner boxed `BusDeviceInstance`.
    pub inner: Box<dyn BusDeviceInstance<F>>,
}

impl<F: PrimeField> BusDeviceInstanceWrapper<F> {
    /// Creates a new `BusDeviceInstanceWrapper` with the given inner `BusDeviceInstance`.
    ///
    /// # Arguments
    /// * `inner` - A boxed implementation of the `BusDeviceInstance` trait.
    ///
    /// # Returns
    /// A new `BusDeviceInstanceWrapper` instance.
    pub fn new(inner: Box<dyn BusDeviceInstance<F>>) -> Self {
        Self { inner }
    }
}

impl<F: PrimeField> BusDevice<u64> for BusDeviceInstanceWrapper<F> {
    /// Processes data received on the bus, delegating the processing to the inner
    /// `BusDeviceInstance`.
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

    /// Returns the bus IDs associated with the inner `BusDeviceInstance`.
    /// This method delegates the call to the inner `BusDeviceInstance`.
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        self.inner.bus_id()
    }
}
