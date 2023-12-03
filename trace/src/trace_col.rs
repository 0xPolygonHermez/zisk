#[derive(Debug)]
pub struct TraceCol<T> {
    pub col: Vec<T>,
}

impl<T: Default + Clone> TraceCol<T> {
    pub fn new(num_rows: usize) -> Self {
        // PRECONDITIONS
        // Size must be greater than 0
        assert!(num_rows >= 2);

        Self { col: vec![T::default(); num_rows] }
    }

    pub fn push(&mut self, value: T) {
        self.col.push(value);
    }

    pub fn get(&self, index: usize) -> &T {
        &self.col[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.col[index]
    }

    pub fn num_rows(&self) -> usize {
        self.col.len()
    }
}

use std::ops::Index;
use std::ops::IndexMut;

impl<T> Index<usize> for TraceCol<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.col[index]
    }
}

impl<T> IndexMut<usize> for TraceCol<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.col[index]
    }
}