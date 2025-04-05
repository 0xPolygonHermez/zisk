use num_bigint::BigInt;
const LAST_CHUNK_OVERLOAD_FACTOR: usize = 16; // times chunk, no bits )2^16 => 2^20)

pub struct ConstantValue {
    pub value: BigInt,
    pub is_hex: bool,
    pub chunks: Vec<BigInt>,
}

/// Represents a constant value that can be split into chunks for easier manipulation.
impl ConstantValue {
    /// Creates a new `ConstantValue` instance.
    ///
    /// # Arguments
    ///
    /// * `value` - The original value to be split into chunks.
    /// * `chunk_size` - The size of each chunk.
    /// * `max_chunks` - The maximum number of chunks.
    /// * `is_hex` - A flag indicating if the value is in hexadecimal format.
    ///
    /// # Returns
    ///
    /// A new `ConstantValue` instance with the value split into chunks.
    ///
    /// # Panics
    ///
    /// This function will panic if the last chunk value exceeds LAST_CHUNK_OVERLOAD_FACTOR
    pub fn new(value: &BigInt, chunk_size: &BigInt, max_chunks: usize, is_hex: bool) -> Self {
        let mut chunks = Vec::new();
        let mut remaining = value.clone();
        let mut available = max_chunks;
        while remaining != BigInt::ZERO && available > 0 {
            chunks.push(if available == 1 {
                assert!(remaining <= (chunk_size * LAST_CHUNK_OVERLOAD_FACTOR));
                &remaining + 0
            } else {
                &remaining % chunk_size
            });
            available -= 1;
            remaining = &remaining / chunk_size;
        }
        Self { value: value.clone(), chunks, is_hex }
    }

    /// Retrieves the chunk at the specified index.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the chunk to retrieve.
    ///
    /// # Returns
    ///
    /// The chunk at the specified index, or `BigInt::ZERO` if the index is out of bounds.
    pub fn get_chunk(&self, idx: usize) -> BigInt {
        self.chunks.get(idx).unwrap_or(&BigInt::ZERO).clone()
    }
}
