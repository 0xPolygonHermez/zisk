use std::collections::BTreeMap;

use crate::{get_ops_costs, MemoryOperationsStats, MAIN_COST};

#[derive(Clone, Debug)]
pub struct CallerInfo {
    pub calls: usize,
    pub steps: usize,
}

#[derive(Clone, Debug)]
pub struct RegionsOfInterest {
    pub from_pc: u32,
    pub to_pc: u32,
    pub steps: u64,
    pub mops: MemoryOperationsStats,
    pub ops: [u64; 256],
    pub cost: u64,
    pub name: String,
    pub calls: usize,
    pub callers: BTreeMap<usize, CallerInfo>,
    pub last_caller_index: Option<usize>,
}

impl RegionsOfInterest {
    pub fn new(from_pc: u32, to_pc: u32, name: &str) -> Self {
        Self {
            from_pc,
            to_pc,
            steps: 0,
            calls: 0,
            mops: MemoryOperationsStats::new(),
            ops: [0u64; 256],
            cost: 0,
            name: name.to_string(),
            callers: BTreeMap::new(),
            last_caller_index: None,
        }
    }
    pub fn contains(&self, pc: u32) -> bool {
        pc >= self.from_pc && pc <= self.to_pc
    }
    pub fn call(&mut self, caller: Option<usize>) {
        self.calls += 1;
        if let Some(caller_id) = caller {
            self.callers
                .entry(caller_id)
                .and_modify(|info| {
                    info.calls += 1;
                    info.steps += 1;
                })
                .or_insert(CallerInfo { calls: 1, steps: 1 });
            self.last_caller_index = Some(caller_id);
        }
    }
    pub fn inc_step(&mut self) {
        self.steps += 1;
        if let Some(index) = self.last_caller_index {
            self.callers.entry(index).and_modify(|info| {
                info.steps += 1;
            });
        }
    }
    pub fn get_callers(&self) -> impl Iterator<Item = (&usize, &CallerInfo)> {
        self.callers.iter()
    }
    pub fn add_op(&mut self, op: u8) {
        self.ops[op as usize] += 1;
    }
    pub fn update_costs(&mut self) {
        let (cost, precompiles_cost) = get_ops_costs(&self.ops);
        self.cost = cost + precompiles_cost + self.mops.get_cost() + self.steps * MAIN_COST;
    }
    pub fn get_cost(&self) -> u64 {
        self.cost
    }
    pub fn get_mem_cost(&self) -> u64 {
        self.mops.get_cost()
    }
    pub fn get_steps(&self) -> u64 {
        self.steps
    }
    pub fn memory_write(&mut self, address: u64, width: u64, value: u64) {
        self.mops.memory_write(address, width, value);
    }
    pub fn memory_read(&mut self, address: u64, width: u64) {
        self.mops.memory_read(address, width);
    }
}
