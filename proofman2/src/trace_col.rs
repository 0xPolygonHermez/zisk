/// A column in a trace
#[derive(Debug, Clone)]
pub struct TraceCol<T> {
    pub col: Vec<T>,
}

impl<T: Default + Clone> TraceCol<T> {
    /// Creates a new TraceCol with the specified number of rows.
    ///
    /// # Arguments
    ///
    /// * `num_rows` - The number of rows in the TraceCol.
    ///
    /// # Preconditions
    ///
    /// * Size must be greater than or equal to 2.
    pub fn new(num_rows: usize) -> Self {
        // PRECONDITIONS
        // Size must be greater than 2
        assert!(num_rows >= 2);

        Self {
            col: vec![T::default(); num_rows],
        }
    }

    /// Gets a reference to the value at the specified index in the TraceCol.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the value to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a reference to the value at the specified index.
    pub fn get(&self, index: usize) -> &T {
        &self.col[index]
    }


    /// Gets a mutable reference to the value at the specified index in the TraceCol.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the value to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the value at the specified index.
    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.col[index]
    }

    /// Gets the number of rows in the TraceCol.
    ///
    /// # Returns
    ///
    /// Returns the number of rows in the TraceCol.
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
