use std::ops::Deref;

/// Type representing a chunk identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkId(pub usize);

impl PartialEq<usize> for ChunkId {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl Deref for ChunkId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
