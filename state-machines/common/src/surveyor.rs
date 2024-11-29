use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    ops::{Add, AddAssign},
};
use zisk_core::{InstContext, ZiskInst};

#[derive(Debug)]
pub enum Survey {
    SurveyCounter(SurveyCounter),
    SurveyStats(SurveyStats),
}

pub trait Surveyor: Debug + Send + Sync + Any {
    fn survey(&mut self, inst: &ZiskInst, inst_ctx: &InstContext);
    fn add(&mut self, other: &dyn Surveyor);
    fn as_any(&self) -> &dyn Any;
}

pub struct DummySurveyor;

impl Surveyor for DummySurveyor {
    fn survey(&mut self, _: &ZiskInst, _: &InstContext) {}
    fn add(&mut self, _: &dyn Surveyor) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for DummySurveyor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DummySurveyor")
    }
}

#[derive(Default, Debug, Clone)]
pub struct SurveyCounter {
    pub inst_count: usize,
}

impl SurveyCounter {
    pub fn update(&mut self, num: usize) {
        self.inst_count += num;
    }
}

impl Add for SurveyCounter {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        SurveyCounter { inst_count: self.inst_count + other.inst_count }
    }
}

impl AddAssign for SurveyCounter {
    fn add_assign(&mut self, other: Self) {
        self.inst_count += other.inst_count;
    }
}

#[derive(Default, Debug, Clone)]
pub struct SurveyStats {
    pub inst_count: HashMap<u64, usize>,
}

impl SurveyStats {
    pub fn update(&mut self, pc: u64, num: usize) {
        let count = self.inst_count.entry(pc).or_default();
        *count += num;
    }
}

impl Add for SurveyStats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut inst_count = self.inst_count.clone();
        for (k, v) in other.inst_count {
            let count = inst_count.entry(k).or_default();
            *count += v;
        }
        SurveyStats { inst_count }
    }
}

impl AddAssign for SurveyStats {
    fn add_assign(&mut self, other: Self) {
        for (k, v) in other.inst_count {
            let count = self.inst_count.entry(k).or_default();
            *count += v;
        }
    }
}
