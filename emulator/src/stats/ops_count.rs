use std::ops::{Add, AddAssign, Sub};

use super::ops_costs::TABLE;

#[derive(Debug, Clone)]
pub struct OpsCount<const N: usize> {
    count: Vec<[u64; N]>,
}

impl<const N: usize> Default for OpsCount<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> OpsCount<N> {
    pub fn new() -> Self {
        Self { count: Vec::new() }
    }
    #[inline(always)]
    pub fn init(&mut self) {
        if self.count.is_empty() {
            self.count = vec![[0; N]; TABLE.base_count];
        }
    }
    pub fn inc(&mut self, op_code: u8, category: usize) {
        self.init();
        if let Some((index, _t)) = TABLE.table[op_code as usize] {
            self.count[index][category] += 1;
        }
    }
    pub fn get_by_opcode(&self, op_code: u8) -> Option<&[u64; N]> {
        if let Some((index, _t)) = TABLE.table[op_code as usize] {
            self.count.get(index)
        } else {
            None
        }
    }
}

impl<const N: usize> Add for OpsCount<N> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if other.count.is_empty() {
            self.clone()
        } else if self.count.is_empty() {
            other.clone()
        } else {
            let mut result = self.clone();
            assert!(
                result.count.len() == other.count.len(),
                "OpsCount addition: count lengths do not match"
            );
            for i in 0..result.count.len() {
                for j in 0..N {
                    result.count[i][j] += other.count[i][j];
                }
            }
            result
        }
    }
}

impl<const N: usize> AddAssign for OpsCount<N> {
    fn add_assign(&mut self, other: Self) {
        if other.count.is_empty() {
            return;
        }
        if self.count.is_empty() {
            self.count = other.count.clone();
        } else {
            assert!(
                self.count.len() == other.count.len(),
                "OpsCount addition: count lengths do not match"
            );
            for i in 0..self.count.len() {
                for j in 0..N {
                    self.count[i][j] += other.count[i][j];
                }
            }
        }
    }
}

impl<const N: usize> Sub for OpsCount<N> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        if other.count.is_empty() {
            if self.count.is_empty() {
                Self::new()
            } else {
                self.clone()
            }
        } else {
            assert!(
                !self.count.is_empty(),
                "OpsCount subtraction: cannot subtract non empty OpsCount from an empty OpsCount"
            );
            let mut result = self.clone();
            assert!(
                result.count.len() == other.count.len(),
                "OpsCount subtraction: count lengths do not match"
            );
            for i in 0..result.count.len() {
                for j in 0..N {
                    result.count[i][j] -= other.count[i][j];
                }
            }
            result
        }
    }
}
