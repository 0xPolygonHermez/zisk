use std::{
    ops::{Add, AddAssign, Sub},
    sync::LazyLock,
};

use zisk_core::zisk_ops::ZiskOp;

struct OpIndirectTable {
    base_count: usize,
    precompiled_count: usize,
    table: [Option<(usize, u64)>; 256],
}

static TABLE: LazyLock<OpIndirectTable> = LazyLock::new(|| {
    let mut table: [Option<(usize, u64)>; 256] = [None; 256];
    let mut base_count = 0;
    let mut precompiled_count = 0;
    for (i, entry) in table.iter_mut().enumerate() {
        if let Ok(instr) = ZiskOp::try_from_code(i as u8) {
            if !instr.is_precompiled() {
                *entry = Some((base_count, instr.cost()));
                base_count += 1;
            }
        }
    }
    for (i, entry) in table.iter_mut().enumerate() {
        if let Ok(instr) = ZiskOp::try_from_code(i as u8) {
            if instr.is_precompiled() {
                *entry = Some((base_count + precompiled_count, instr.cost()));
                precompiled_count += 1;
            }
        }
    }
    OpIndirectTable { base_count, precompiled_count, table }
});

#[derive(Debug, Clone)]
pub struct OpsCosts {
    // for count and cost for operation or group of operation only store the count, the cost is calculated multiplying
    // each operation for its cost
    count_and_cost: Vec<(usize, u64)>,
    base_cost: u64,
    precompiled_cost: u64,
    base_count: usize,
    precompiled_count: usize,
}

impl OpsCosts {
    pub fn new(compact: bool, is_frops: bool) -> Self {
        Self {
            count_and_cost: if compact {
                Vec::new()
            } else {
                vec![
                    (0, 0);
                    if is_frops {
                        TABLE.base_count
                    } else {
                        TABLE.base_count + TABLE.precompiled_count
                    }
                ]
            },
            base_cost: 0,
            precompiled_cost: 0,
            base_count: 0,
            precompiled_count: 0,
        }
    }
    pub fn add_fixed_cost_op(&mut self, op_code: u8) {
        if let Some((index, fixed_cost)) = TABLE.table[op_code as usize] {
            if fixed_cost > 0 {
                if !self.is_compact() {
                    self.count_and_cost[index].0 += 1;
                    self.count_and_cost[index].1 += fixed_cost;
                }
                if index >= TABLE.base_count {
                    self.precompiled_cost += fixed_cost;
                    self.precompiled_count += 1;
                } else {
                    self.base_cost += fixed_cost;
                    self.base_count += 1;
                }
            }
        } else {
            panic!("Invalid op code: {}", op_code);
        }
    }
    pub fn add_variable_cost_op(&mut self, op_code: u8, variable_cost: u64) {
        if let Some((index, fixed_cost)) = TABLE.table[op_code as usize] {
            if !self.is_compact() {
                self.count_and_cost[index].0 += 1;
                self.count_and_cost[index].1 += fixed_cost + variable_cost;
            }
            if index >= TABLE.base_count {
                self.precompiled_cost += fixed_cost + variable_cost;
                self.precompiled_count += 1;
            } else {
                self.base_cost += fixed_cost + variable_cost;
                self.base_count += 1;
            }
        } else {
            panic!("Invalid op code: {}", op_code);
        }
    }
    pub fn total_cost(&self) -> u64 {
        self.base_cost + self.precompiled_cost
    }
    pub fn base_cost(&self) -> u64 {
        self.base_cost
    }
    pub fn precompiled_cost(&self) -> u64 {
        self.precompiled_cost
    }
    pub fn total_count(&self) -> usize {
        self.base_count + self.precompiled_count
    }
    pub fn base_count(&self) -> usize {
        self.base_count
    }
    pub fn precompiled_count(&self) -> usize {
        self.precompiled_count
    }

    #[inline(always)]
    pub fn is_compact(&self) -> bool {
        self.count_and_cost.is_empty()
    }
    #[inline(always)]
    pub fn is_frops(&self) -> bool {
        self.count_and_cost.len() == TABLE.base_count
    }

    /// Creates a compact (non-detailed) clone with only summary data
    pub fn clone_compact(&self) -> Self {
        Self {
            count_and_cost: Vec::new(), // Empty = no detailed data
            base_cost: self.base_cost,
            precompiled_cost: self.precompiled_cost,
            base_count: self.base_count,
            precompiled_count: self.precompiled_count,
        }
    }

    pub fn add_delta(&mut self, reference: &OpsCosts, current: &OpsCosts) {
        for i in 0..self.count_and_cost.len() {
            self.count_and_cost[i].0 += current.count_and_cost[i].0 - reference.count_and_cost[i].0;
            self.count_and_cost[i].1 += current.count_and_cost[i].1 - reference.count_and_cost[i].1;
        }
        self.base_cost += current.base_cost - reference.base_cost;
        self.precompiled_cost += current.precompiled_cost - reference.precompiled_cost;
        self.base_count += current.base_count - reference.base_count;
        self.precompiled_count += current.precompiled_count - reference.precompiled_count;
    }
    pub fn get_opcode_count_and_cost(&self, op_code: u8) -> Option<(usize, u64)> {
        if !self.is_compact() {
            if let Some((index, _)) = TABLE.table[op_code as usize] {
                if index < self.count_and_cost.len() {
                    return Some(self.count_and_cost[index]);
                }
            }
        }
        None
    }

    /// Returns the top K opcodes ranked by cost (highest to lowest)
    ///
    /// # Arguments
    /// * `k` - Maximum number of opcodes to return in the ranking
    ///
    /// # Returns
    /// A vector of opcodes (u8) sorted by cost in descending order, limited to k elements
    pub fn top_cost_opcodes(&self, k: usize) -> Vec<u8> {
        if self.is_compact() || k == 0 {
            return Vec::new();
        }

        // Use a min-heap to keep only top k elements
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        let mut heap: BinaryHeap<Reverse<(u64, u8)>> = BinaryHeap::with_capacity(k + 1);

        for op_code in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
            if let Some((index, _)) = TABLE.table[op_code as usize] {
                if self.is_frops() && index >= TABLE.base_count {
                    continue; // Skip non-frops if this is a frops cost
                }
                let cost = self.count_and_cost[index].1;
                if cost > 0 {
                    heap.push(Reverse((cost, op_code)));
                    if heap.len() > k {
                        heap.pop();
                    }
                }
            }
        }

        // Extract and reverse to get descending order
        let mut result: Vec<(u64, u8)> = heap.into_iter().map(|Reverse(x)| x).collect();
        result.sort_by(|a, b| b.0.cmp(&a.0));
        result.iter().map(|(_, opcode)| *opcode).collect()
    }

    /// Returns the top K opcodes ranked by count (highest to lowest)
    ///
    /// # Arguments
    /// * `k` - Maximum number of opcodes to return in the ranking
    ///
    /// # Returns
    /// A vector of opcodes (u8) sorted by count in descending order, limited to k elements
    pub fn top_count_opcodes(&self, k: usize) -> Vec<u8> {
        if self.is_compact() || k == 0 {
            return Vec::new();
        }

        // Use a min-heap to keep only top k elements
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        let mut heap: BinaryHeap<Reverse<(usize, u8)>> = BinaryHeap::with_capacity(k + 1);

        for op_code in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
            if let Some((index, _)) = TABLE.table[op_code as usize] {
                if self.is_frops() && index >= TABLE.base_count {
                    continue; // Skip non-frops if this is a frops cost
                }
                let count = self.count_and_cost[index].0;
                if count > 0 {
                    heap.push(Reverse((count, op_code)));
                    if heap.len() > k {
                        heap.pop();
                    }
                }
            }
        }

        // Extract and reverse to get descending order
        let mut result: Vec<(usize, u8)> = heap.into_iter().map(|Reverse(x)| x).collect();
        result.sort_by(|a, b| b.0.cmp(&a.0));
        result.iter().map(|(_, opcode)| *opcode).collect()
    }
}

impl Add for OpsCosts {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let is_compact = self.is_compact() || other.is_compact();
        let is_frops = self.is_frops() || other.is_frops();
        let mut result = Self::new(is_compact, is_frops);
        for i in 0..result.count_and_cost.len() {
            result.count_and_cost[i].0 = self.count_and_cost[i].0 + other.count_and_cost[i].0;
            result.count_and_cost[i].1 = self.count_and_cost[i].1 + other.count_and_cost[i].1;
        }
        result.base_cost = self.base_cost + other.base_cost;
        result.precompiled_cost = self.precompiled_cost + other.precompiled_cost;
        result.base_count = self.base_count + other.base_count;
        result.precompiled_count = self.precompiled_count + other.precompiled_count;
        result
    }
}

impl AddAssign for OpsCosts {
    fn add_assign(&mut self, other: Self) {
        for i in 0..self.count_and_cost.len() {
            self.count_and_cost[i].0 += other.count_and_cost[i].0;
            self.count_and_cost[i].1 += other.count_and_cost[i].1;
        }
        self.base_cost += other.base_cost;
        self.precompiled_cost += other.precompiled_cost;
        self.base_count += other.base_count;
        self.precompiled_count += other.precompiled_count;
    }
}

impl Sub for OpsCosts {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let is_compact = self.is_compact() || other.is_compact();
        let is_frops = self.is_frops() || other.is_frops();
        let mut result = Self::new(is_compact, is_frops);
        for i in 0..result.count_and_cost.len() {
            result.count_and_cost[i].0 = self.count_and_cost[i].0 - other.count_and_cost[i].0;
            result.count_and_cost[i].1 = self.count_and_cost[i].1 - other.count_and_cost[i].1;
        }
        result.base_cost = self.base_cost - other.base_cost;
        result.precompiled_cost = self.precompiled_cost - other.precompiled_cost;
        result.base_count = self.base_count - other.base_count;
        result.precompiled_count = self.precompiled_count - other.precompiled_count;
        result
    }
}
