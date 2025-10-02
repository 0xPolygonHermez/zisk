use std::{
    cmp::Ordering,
    fmt::{self, Display},
    ops::{Deref, DerefMut},
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct U256([u64; 4]); // little-endian: 4 × 64 = 256 bits

impl U256 {
    pub const ZERO: Self = U256([0, 0, 0, 0]);
    pub const ONE: Self = U256([1, 0, 0, 0]);
    pub const TWO: Self = U256([2, 0, 0, 0]);
    pub const MAX: Self = U256([u64::MAX, u64::MAX, u64::MAX, u64::MAX]);

    #[inline]
    pub const fn from_u64s(a: &[u64; 4]) -> Self {
        U256(*a)
    }

    #[inline]
    pub const fn from_u64(a: u64) -> Self {
        U256([a, 0, 0, 0])
    }

    /// Convert a slice of U256 to a flat slice of u64
    #[inline]
    pub fn slice_to_flat(slice: &[U256]) -> &[u64] {
        unsafe { core::slice::from_raw_parts(slice.as_ptr() as *const u64, slice.len() * 4) }
    }

    /// Reconstruct a slice of U256 from a flat slice of u64
    #[inline]
    pub fn slice_from_flat(flat: &[u64]) -> Vec<U256> {
        assert!(flat.len() % 4 == 0, "Flat slice length must be multiple of 4");
        flat.chunks_exact(4).map(|chunk| U256([chunk[0], chunk[1], chunk[2], chunk[3]])).collect()
    }

    #[inline]
    pub fn is_odd(&self) -> bool {
        self.0[0] & 1 == 1
    }
}

impl Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}{:016x}{:016x}{:016x}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl PartialEq for U256 {
    fn eq(&self, other: &Self) -> bool {
        self.0[3] == other.0[3]
            && self.0[2] == other.0[2]
            && self.0[1] == other.0[1]
            && self.0[0] == other.0[0]
    }
}

impl Eq for U256 {}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare from most significant limb
        for i in (0..4).rev() {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Equal => continue,
                other => return other,
            }
        }
        Ordering::Equal
    }
}

impl Deref for U256 {
    type Target = [u64; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for U256 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
