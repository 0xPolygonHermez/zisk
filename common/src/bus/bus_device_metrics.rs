//! The `BusDeviceMetrics` and `BusDeviceMetricsWrapper` modules integrate the functionalities
//! of `BusDevice` and `Metrics`, providing a unified interface for monitoring and managing
//! bus operations with associated metrics.

use super::BusDevice;

use crate::Metrics;

#[derive(Debug, PartialEq)]
pub enum BusDeviceMode {
    Counter,
    CounterAsm,
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
