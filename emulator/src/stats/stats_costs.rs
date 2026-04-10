use crate::{MemoryOperationsStats, OpsCosts, MAIN_COST};
#[derive(Debug, Clone)]
pub struct StatsCosts {
    pub steps: u64,
    pub mops: MemoryOperationsStats,
    ops: OpsCosts,
    frops_ops: OpsCosts,
    pub cost: u64,
}

impl StatsCosts {
    pub fn new_no_compact() -> Self {
        Self {
            steps: 0,
            mops: MemoryOperationsStats::new(),
            ops: OpsCosts::new(false, false),
            frops_ops: OpsCosts::new(false, true),
            cost: 0,
        }
    }
    pub fn new_compact() -> Self {
        Self {
            steps: 0,
            mops: MemoryOperationsStats::new(),
            ops: OpsCosts::new(true, false),
            frops_ops: OpsCosts::new(true, true),
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
    pub fn add_delta(
        &mut self,
        reference: &StatsCosts,
        current: &StatsCosts,
    ) -> Result<u64, String> {
        let delta_steps = current.steps - reference.steps - 1;
        if self.steps >= reference.steps && reference.steps > 0 {
            return Err(format!(
                "COSTS OVERFLOW STEPS:{} + DELTA:{} => STEPS'{} (REF: {})",
                self.steps,
                delta_steps,
                self.steps + delta_steps,
                current.steps
            ));
        }
        self.steps += delta_steps;
        self.cost += current.cost - reference.cost;
        self.ops.add_delta(&reference.ops, &current.ops);
        self.frops_ops.add_delta(&reference.frops_ops, &current.frops_ops);
        self.mops.add_delta(&reference.mops, &current.mops);
        Ok(delta_steps)
    }
    // steps, ops costs, precompiles costs, memory costs
    pub fn summary(&self) -> (u64, u64, u64, u64) {
        (self.steps, self.ops.base_cost(), self.ops.precompiled_cost(), self.mops.get_cost())
    }

    /// Creates a compact (non-detailed) clone with only summary data
    pub fn clone_compact(&self) -> Self {
        Self {
            steps: self.steps,
            mops: self.mops.clone(),
            ops: self.ops.clone_compact(),
            frops_ops: self.frops_ops.clone_compact(),
            cost: self.cost,
        }
    }
    #[inline(always)]
    pub fn get_opcode_count_and_cost(&self, op_code: u8) -> Option<(usize, u64)> {
        self.ops.get_opcode_count_and_cost(op_code)
    }
    #[inline(always)]
    pub fn get_opcode_frops_count_and_cost(&self, op_code: u8) -> Option<(usize, u64)> {
        self.frops_ops.get_opcode_count_and_cost(op_code)
    }
    #[inline(always)]
    pub fn top_count_opcodes(&self, k: usize) -> Vec<u8> {
        self.ops.top_count_opcodes(k)
    }
    #[inline(always)]
    pub fn top_count_frops_opcodes(&self, k: usize) -> Vec<u8> {
        self.frops_ops.top_count_opcodes(k)
    }
    #[inline(always)]
    pub fn top_cost_opcodes(&self, k: usize) -> Vec<u8> {
        self.ops.top_cost_opcodes(k)
    }
    #[inline(always)]
    pub fn top_cost_frops_opcodes(&self, k: usize) -> Vec<u8> {
        self.frops_ops.top_cost_opcodes(k)
    }
    #[inline(always)]
    pub fn ops_costs(&self) -> &OpsCosts {
        &self.ops
    }
    #[inline(always)]
    pub fn frops_costs(&self) -> &OpsCosts {
        &self.frops_ops
    }
    #[inline(always)]
    pub fn add_fixed_cost_op(&mut self, op_code: u8) {
        self.ops.add_fixed_cost_op(op_code);
    }
    #[inline(always)]
    pub fn add_variable_cost_op(&mut self, op_code: u8, variable_cost: u64) {
        self.ops.add_variable_cost_op(op_code, variable_cost);
    }
    #[inline(always)]
    pub fn add_fixed_frops_cost_op(&mut self, op_code: u8) {
        self.frops_ops.add_fixed_cost_op(op_code);
    }
    #[inline(always)]
    pub fn total_cost(&self) -> u64 {
        self.ops.total_cost() + self.mops.get_cost() + self.steps * MAIN_COST
    }
    #[inline(always)]
    pub fn total_ops_cost(&self) -> u64 {
        self.ops.total_cost()
    }
    #[inline(always)]
    pub fn base_ops_cost(&self) -> u64 {
        self.ops.base_cost()
    }
    #[inline(always)]
    pub fn precompiled_ops_cost(&self) -> u64 {
        self.ops.precompiled_cost()
    }
    #[inline(always)]
    pub fn frops_cost(&self) -> u64 {
        self.frops_ops.total_cost()
    }
}
