use crate::{get_ops_costs, MemoryOperationsStats};
#[derive(Clone, Debug)]
pub struct StatsCosts {
    pub steps: u64,
    pub mops: MemoryOperationsStats,
    pub ops: [u64; 256],
    pub frops_ops: [u64; 256],
    pub cost: u64,
}

impl StatsCosts {
    pub fn new() -> Self {
        Self {
            steps: 0,
            mops: MemoryOperationsStats::new(),
            ops: [0u64; 256],
            frops_ops: [0u64; 256],
            cost: 0,
        }
    }
    pub fn memory_write(&mut self, address: u64, width: u64, value: u64) {
        self.mops.memory_write(address, width, value);
    }
    pub fn memory_read(&mut self, address: u64, width: u64) {
        self.mops.memory_read(address, width);
    }
    pub fn get_delta_steps(&mut self, reference: &StatsCosts, current: &StatsCosts) -> u64 {
        current.steps - reference.steps - 1
    }
    pub fn add_delta(&mut self, reference: &StatsCosts, current: &StatsCosts) -> u64 {
        let delta_steps = current.steps - reference.steps - 1;
        self.steps += delta_steps;
        self.cost += current.cost - reference.cost;
        for i in 0..256 {
            self.ops[i] += current.ops[i] - reference.ops[i];
            self.frops_ops[i] += current.frops_ops[i] - reference.frops_ops[i];
        }
        self.mops.add_delta(&reference.mops, &current.mops);
        delta_steps
    }
    // steps, ops costs, precompiles costs, memory costs
    pub fn summary(&self) -> (u64, u64, u64, u64) {
        let ops_costs = get_ops_costs(&self.ops);
        (self.steps, ops_costs.0, ops_costs.1, self.mops.get_cost())
    }
}

impl Default for StatsCosts {
    fn default() -> Self {
        Self::new()
    }
}
