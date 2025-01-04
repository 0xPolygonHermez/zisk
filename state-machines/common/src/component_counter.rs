use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    ops::{Add, AddAssign},
};
use zisk_common::BusId;

#[derive(Debug)]
pub enum CounterType {
    Counter(Counter),
    CounterStats(CounterStats),
}

pub trait Metrics: Send + Sync {
    fn measure(&mut self, bus_id: &BusId, data: &[u64]) -> Vec<(BusId, Vec<u64>)>;

    fn add(&mut self, other: &dyn Metrics);

    fn bus_id(&self) -> Vec<BusId>;

    fn on_close(&mut self) {}

    fn as_any(&self) -> &dyn Any;
}

#[derive(Default, Debug, Clone)]
pub struct Counter {
    /// Counted instructions
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
    /// Hash map of counted instructions by PC (key: PC, value: number of counted instructions)
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
