pub const CHUNKS: usize = 8;
pub const CHUNK_BITS: usize = 32;
pub const CHUNK_MASK: usize = (1 << CHUNK_BITS) - 1;

enum CallbackType<T> {
    Single(Box<dyn Fn() -> T>),
    Array(Box<dyn Fn() -> [T; CHUNKS]>),
}
