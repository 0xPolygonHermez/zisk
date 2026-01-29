use std::collections::BTreeMap;

use crate::{get_ops_costs, StatsCosts, MAIN_COST};

#[derive(Clone, Debug)]
pub struct CallerInfo {
    pub calls: usize,
    pub steps: usize,
}

#[derive(Clone, Debug)]
pub struct RegionsOfInterest {
    pub id: usize,
    pub from_pc: u32,
    pub to_pc: u32,
    pub name: String,
    costs: StatsCosts,
    pub calls: usize,
    pub callers: BTreeMap<usize, CallerInfo>,
    pub call_stack_rc: usize,
    call_stack_depth: Option<usize>,
}

impl RegionsOfInterest {
    pub fn new(id: usize, from_pc: u32, to_pc: u32, name: &str) -> Self {
        Self {
            id,
            from_pc,
            to_pc,
            costs: StatsCosts::new(),
            calls: 0,
            name: name.to_string(),
            callers: BTreeMap::new(),
            call_stack_rc: 0,
            call_stack_depth: None,
        }
    }
    pub fn contains(&self, pc: u32) -> bool {
        pc >= self.from_pc && pc <= self.to_pc
    }
    pub fn caller_call(&mut self) {
        self.call_stack_rc += 1;
    }
    pub fn update_call_depth(&mut self, call_stack_depth: usize) {
        if let Some(depth) = self.call_stack_depth {
            self.call_stack_depth = Some(std::cmp::min(depth, call_stack_depth));
        } else {
            self.call_stack_depth = Some(call_stack_depth);
        }
    }
    pub fn call(&mut self, caller: Option<usize>, call_stack_depth: usize) {
        self.calls += 1;
        self.update_call_depth(call_stack_depth);
        if let Some(caller_id) = caller {
            self.callers
                .entry(caller_id)
                .and_modify(|info| {
                    info.calls += 1;
                })
                .or_insert(CallerInfo { calls: 1, steps: 0 });
        }
    }
    pub fn tail_jmp(&mut self, source: Option<usize>) {
        if let Some(source_id) = source {
            self.callers
                .entry(source_id)
                .and_modify(|info| {
                    info.calls += 1;
                })
                .or_insert(CallerInfo { calls: 1, steps: 0 });
        }
    }
    pub fn return_call(&mut self, call_stack_depth: usize) {
        let rc = self.call_stack_rc;
        if self.call_stack_rc > 0 {
            self.call_stack_rc -= 1;
        }
        self.update_call_depth(call_stack_depth);
        assert!(rc > self.call_stack_rc);
    }
    pub fn get_callers(&self) -> impl Iterator<Item = (&usize, &CallerInfo)> {
        self.callers.iter()
    }
    pub fn update_costs(&mut self) {
        let (cost, precompiles_cost) = get_ops_costs(&self.costs.ops);
        self.costs.cost =
            cost + precompiles_cost + self.costs.mops.get_cost() + self.costs.steps * MAIN_COST;
    }
    pub fn get_cost(&self) -> u64 {
        self.costs.cost
    }
    pub fn get_mem_cost(&self) -> u64 {
        self.costs.mops.get_cost()
    }
    pub fn get_steps(&self) -> u64 {
        self.costs.steps
    }
    pub fn get_callstack_rc(&self) -> usize {
        self.call_stack_rc
    }
    pub fn get_ops_costs(&self) -> &[u64; 256] {
        &self.costs.ops
    }
    pub fn get_call_stack_depth(&self) -> Option<usize> {
        self.call_stack_depth
    }
    pub fn add_delta_costs(&mut self, reference: &StatsCosts, current: &StatsCosts) -> u64 {
        if self.call_stack_rc == 0 {
            self.costs.add_delta(reference, current)
        } else {
            self.costs.get_delta_steps(reference, current)
        }
    }
    pub fn get_delta_steps(&mut self, reference: &StatsCosts, current: &StatsCosts) -> u64 {
        self.costs.get_delta_steps(reference, current)
    }
    pub fn update_caller_steps(&mut self, caller_id: usize, steps: u64) {
        self.callers.entry(caller_id).and_modify(|info| {
            info.steps += steps as usize;
        });
    }
    pub fn update_caller(
        &mut self,
        caller_id: usize,
        reference: &StatsCosts,
        current: &StatsCosts,
    ) {
        let steps = self.get_delta_steps(reference, current);
        self.callers.entry(caller_id).and_modify(|info| {
            info.steps += steps as usize;
        });
    }
}
