//! The `DummyCounter` module defines a placeholder counter that performs no operations.
//!
//! This counter is used as a default implementation when no actual counting or metrics
//! collection is required.

use std::{any::Any, collections::VecDeque};

use zisk_common::{BusDevice, BusId, Metrics};

/// The `DummyCounter` struct serves as a placeholder counter that performs no actions
/// when connected to the data bus.
///
/// It implements the `Metrics` and `BusDevice` traits but does not track, update, or return
/// any metrics or inputs.
#[derive(Default)]
pub struct DummyCounter {}

impl Metrics for DummyCounter {
    /// Does nothing when tracking activity on the bus.
    ///
    /// # Arguments
    /// * `_data` - The data received from the bus (ignored in this implementation).
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any metrics.
    #[inline(always)]
    fn measure(&mut self, _: &[u64]) {}

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BusDevice<u64> for DummyCounter {
    #[inline(always)]
    fn process_data(
        &mut self,
        _bus_id: &BusId,
        _data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        true
    }

    /// Returns an empty vector as this counter is not associated with any bus IDs.
    ///
    /// # Returns
    /// An empty vector of bus IDs.
    fn bus_id(&self) -> Vec<BusId> {
        vec![]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
