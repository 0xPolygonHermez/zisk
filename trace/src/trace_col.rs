#[derive(Debug)]
pub struct TraceCol<T> {
    col: Vec<T>,
}

impl<T: Default + Clone> TraceCol<T> {
    pub fn new(len: usize) -> Self {
        Self { col: vec![T::default(); len] }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self { col: Vec::with_capacity(capacity) }
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

    pub fn len(&self) -> usize {
        self.col.len()
    }
}

use std::ops::Index;

impl<T> Index<usize> for TraceCol<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.col[index]
    }
}