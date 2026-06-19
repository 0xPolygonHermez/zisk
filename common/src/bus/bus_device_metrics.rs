//! The `BusDeviceMetrics` and `BusDeviceMetricsWrapper` modules integrate the functionalities
//! of `BusDevice` and `Metrics`, providing a unified interface for monitoring and managing
//! bus operations with associated metrics.

use super::BusDevice;

use crate::Metrics;

/// Represents the operational mode of a bus device.
#[derive(Debug, PartialEq, Clone)]
pub enum BusDeviceMode {
    /// The device is operating in counter mode.
    Counter,
    /// The device is operating in counter mode with assembly emulation.
    CounterAsm,
    /// The device is operating in input generator mode.
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
