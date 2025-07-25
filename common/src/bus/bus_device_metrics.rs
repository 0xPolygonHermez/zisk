//! The `BusDeviceMetrics` and `BusDeviceMetricsWrapper` modules integrate the functionalities
//! of `BusDevice` and `Metrics`, providing a unified interface for monitoring and managing
//! bus operations with associated metrics.

use std::{any::Any, collections::VecDeque};

use super::{BusDevice, BusId};

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

impl BusDevice<u64> for Box<dyn BusDeviceMetrics> {
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

/// Blanket implementation of `BusDeviceMetrics` for any type implementing `BusDevice`,
/// `Metrics`, and `std::any::Any`.
impl<T: BusDevice<u64> + Metrics + std::any::Any> BusDeviceMetrics for T {}
