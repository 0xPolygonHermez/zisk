use std::ops::Deref;

/// Type representing the payload transmitted across the bus.
pub type PayloadType = u64;

/// Type representing a bus ID.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BusId(pub usize);

impl PartialEq<usize> for BusId {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl Deref for BusId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
