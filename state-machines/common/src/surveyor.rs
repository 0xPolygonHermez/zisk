use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    ops::{Add, AddAssign},
};
use zisk_core::{InstContext, ZiskInst};

#[derive(Debug)]
pub enum CounterType {
    SurveyCounter(Counter),
    SurveyStats(CounterStats),
}

pub trait Metrics: Debug + Send + Sync + Any {
    fn measure(&mut self, inst: &ZiskInst, inst_ctx: &InstContext);
    fn add(&mut self, other: &dyn Metrics);

    fn as_any(&self) -> &dyn Any;
}

#[derive(Default, Debug, Clone)]
pub struct Counter {
    pub inst_count: usize,
}

impl Counter {
    pub fn update(&mut self, num: usize) {
        self.inst_count += num;
    }
}

impl Add for Counter {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Counter { inst_count: self.inst_count + other.inst_count }
    }
}

impl AddAssign for Counter {
    fn add_assign(&mut self, other: Self) {
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

impl Add for CounterStats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut inst_count = self.inst_count.clone();
        for (k, v) in other.inst_count {
            let count = inst_count.entry(k).or_default();
            *count += v;
        }
        CounterStats { inst_count }
    }
}

impl AddAssign for CounterStats {
    fn add_assign(&mut self, other: Self) {
        for (k, v) in other.inst_count {
            let count = self.inst_count.entry(k).or_default();
            *count += v;
        }
    }
}
