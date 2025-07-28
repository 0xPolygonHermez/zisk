/// A per-thread local table for tracking row multiplicities in a binary operation context.
///
/// This structure is designed to reduce contention in multi-threaded environments by storing
/// most multiplicities in a fixed-size array for fast access. The threshold between
/// array and vector storage is determined by `num_bits`.
///
/// Keys less than `2^num_bits` are stored in the `multiplicity` array. Keys equal to or
/// greater than that are stored in the `multiplicity_vec`. Empirical data shows that a high percentage
/// of keys fall below this threshold, making this a memory-efficient and performant trade-off.
/// array and vector storage is determined by `num_bits`.
pub struct LocalTable<const SIZE: usize> {
    /// This is a multiplicity table to use local on each thread to avoid contention.
    /// In the multiplicity field, we store the multiplicity of each row only for the keys that are less than num_bits.
    /// For the keys that are greater than num_bits, we store the multiplicities in a Vector.
    /// Stadistically the 90% of the keys are less than num_bits, so this is a good trade-off.
    pub multiplicity: Box<[u8; SIZE]>,

    /// Vector to store the multiplicity of each row for keys greater than num_bits.
    pub multiplicity_vec: Vec<(u64, u64)>,
}

impl<const SIZE: usize> Default for LocalTable<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const SIZE: usize> LocalTable<SIZE> {
    /// Creates a new `BinaryBasicTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `BinaryBasicTableSM`.
    pub fn new() -> Self {
        Self { multiplicity: Box::new([0; SIZE]), multiplicity_vec: Vec::with_capacity(1) }
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `input` - A slice of `u64` values representing the input data.
    #[inline(always)]
    pub fn update_multiplicity(&mut self, row: u64, value: u64) {
        // I want to know in how many bits can be represented the row value, if it fits in the T type, it means T type is represented in equal or more bits than the row value.
        // it will be stored in multiplicity field. Otherwise push the (row, value) to multiplicity_vec.
        let final_value = self.multiplicity[row as usize] as u64 + value;

        if final_value < u8::MAX as u64 {
            self.multiplicity[row as usize] = final_value as u8;
        } else {
            self.multiplicity_vec.push((row, final_value));
            self.multiplicity[row as usize] = 0;
        }
    }
}
