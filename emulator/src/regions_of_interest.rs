use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};

use crate::{get_ops_costs, StatsCosts, MAIN_COST};

#[derive(Debug)]
pub struct CallerInfo {
    pub calls: usize,
    pub steps: usize,
}

#[derive(Debug)]
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
    pub is_selected_roi: bool,
    pub track_calls: usize,
    tracked_calls: Vec<Vec<u64>>,
    track_file: Option<BufWriter<File>>,
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
            is_selected_roi: false,
            track_calls: 0,
            tracked_calls: Vec::new(),
            track_file: None,
        }
    }

    pub fn set_selected_roi(&mut self, track_calls: usize) {
        self.is_selected_roi = true;
        self.track_calls = track_calls;
    }

    pub fn init_tracking(
        &mut self,
        output_path: &str,
        separator: &str,
        filename: &str,
    ) -> std::io::Result<()> {
        if self.track_calls == 0 {
            return Ok(());
        }

        // Create output directory if it doesn't exist
        fs::create_dir_all(output_path)?;

        let filepath = format!("{}/{}.txt", output_path, filename);
        let file = File::create(&filepath)?;
        let mut writer = BufWriter::new(file);

        // Write header
        writeln!(writer, "# ROI: {} (PC: 0x{:08x}-0x{:08x})", self.name, self.from_pc, self.to_pc)?;
        writeln!(writer, "# Separator: '{}'", separator)?;
        writeln!(writer, "# Parameters: a0-a{}", self.track_calls.min(8) - 1)?;

        self.track_file = Some(writer);
        Ok(())
    }

    pub fn track_call_parameters(&mut self, registers: &[u64], separator: &str, caller: &str) {
        if self.track_calls == 0 {
            return;
        }

        // RISC-V registers a0-a7 are at indices 10-17
        let num_params = self.track_calls.min(8);
        let mut params = Vec::with_capacity(num_params);

        for i in 0..num_params {
            if 10 + i < registers.len() {
                params.push(registers[10 + i]);
            } else {
                params.push(0);
            }
        }

        // Write to file if available
        if let Some(ref mut file) = self.track_file {
            let line = params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(separator);
            if caller.is_empty() {
                let _ = writeln!(file, "{line}");
            } else {
                let _ = writeln!(file, "{line};{caller}");
            }
        }

        self.tracked_calls.push(params);
    }

    pub fn get_tracked_calls(&self) -> &[Vec<u64>] {
        &self.tracked_calls
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
