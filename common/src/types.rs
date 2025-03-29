use std::{fmt, ops::Deref};

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

/// Type representing a chunk identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SegmentId(pub usize);

impl PartialEq<usize> for SegmentId {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl Deref for SegmentId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for SegmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
