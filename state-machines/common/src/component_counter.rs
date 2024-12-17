use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    ops::{Add, AddAssign},
};
use zisk_common::BusId;
use zisk_core::ZiskOperationType;

#[derive(Debug)]
pub enum CounterType {
    Counter(Counter),
    CounterStats(CounterStats),
}

pub trait Metrics: Send + Sync {
    fn measure(&mut self, bus_id: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)>;

    fn add(&mut self, other: &dyn Metrics);

    /// Returns the operation types that this metric is interested in.
    /// This is used to filter out metrics that are not interested while executing the ROM.
    /// If a Metrics is not interested in any operation types, it should return an empty vector.
    /// If a Metrics is interested in all operation types, it should return a vector with a single
    /// element `ZiskOperationType::None`.
    fn op_type(&self) -> Vec<ZiskOperationType>;

    fn bus_id(&self) -> Vec<BusId>;

    fn as_any(&self) -> &dyn Any;
}

#[derive(Default, Debug, Clone)]
pub struct Counter {
    pub inst_count: u64,
}

impl Counter {
    pub fn update(&mut self, num: u64) {
        self.inst_count += num;
    }
}

impl Add for &Counter {
    type Output = Counter;

    fn add(self, other: Self) -> Counter {
        Counter { inst_count: self.inst_count + other.inst_count }
    }
}

impl AddAssign<&Counter> for Counter {
    fn add_assign(&mut self, other: &Counter) {
        self.inst_count += other.inst_count;
    }
}

#[derive(Default, Debug, Clone)]
pub struct CounterStats {
    pub inst_count: HashMap<u64, u64>,
}

impl CounterStats {
    pub fn update(&mut self, pc: u64, num: usize) {
        let count = self.inst_count.entry(pc).or_default();
        *count += num as u64;
    }
}

impl Add for &CounterStats {
    type Output = CounterStats;

    fn add(self, other: Self) -> CounterStats {
        let mut inst_count = self.inst_count.clone();
        for (k, v) in &other.inst_count {
            let count = inst_count.entry(*k).or_default();
            *count += v;
        }
        CounterStats { inst_count }
    }
}

impl AddAssign<&CounterStats> for CounterStats {
    fn add_assign(&mut self, other: &CounterStats) {
        for (k, v) in &other.inst_count {
            let count = self.inst_count.entry(*k).or_default();
            *count += v;
        }
    }
}
