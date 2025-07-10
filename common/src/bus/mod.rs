mod bus_device;
mod bus_device_metrics;
mod bus_id;
mod data_bus_mem;
mod data_bus_operation;
mod data_bus_rom;

use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

pub use bus_device::*;
pub use bus_device_metrics::*;
pub use bus_id::*;
pub use data_bus_mem::*;
pub use data_bus_operation::*;
pub use data_bus_rom::*;

use std::fmt::{Formatter, Result};

#[derive(Debug, Clone, Default)]
pub struct DebugBusTime {
    pub sm_time: [u64; 12],
    pub sm_count: [u64; 12],
}

impl Add for DebugBusTime {
    type Output = DebugBusTime;

    fn add(self, other: DebugBusTime) -> DebugBusTime {
        let mut result = self;

        for i in 0..12 {
            result.sm_time[i] += other.sm_time[i];
            result.sm_count[i] += other.sm_count[i];
        }
        result
    }
}

impl AddAssign for DebugBusTime {
    fn add_assign(&mut self, other: Self) {
        for i in 0..12 {
            self.sm_time[i] += other.sm_time[i];
            self.sm_count[i] += other.sm_count[i];
        }
    }
}

impl Display for DebugBusTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "DebugBusTime Report")?;
        writeln!(f, "====================")?;
        let total_time = self.sm_time.iter().sum::<u64>();

        writeln!(f, "Total time: {} ms", total_time as f64 / 1_000_000.0)?;
        writeln!(
            f,
            "{:<6} {:>15} {:>10} {:>15} {:>10}",
            "SM", "Time (ns)", "Count", "   Avg (ns)", "% Impact"
        )?;

        for (i, (&time, &count)) in self.sm_time.iter().zip(&self.sm_count).enumerate() {
            let avg = if count > 0 { time / count } else { 0 };
            let percent =
                if total_time > 0 { 100.0 * time as f64 / total_time as f64 } else { 0.0 };
            writeln!(f, "SM#{i:<3} {time:>15} {count:>10} {avg:>15} {percent:>9.2}%")?;
        }

        Ok(())
    }
}
