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

    /// Compare two slices of U256 as big integers for equality
    pub fn eq_slices(a: &[U256], b: &[U256]) -> bool {
        let len_a = a.len();
        let len_b = b.len();

        let max_len = len_a.max(len_b);

        for i in (0..max_len).rev() {
            let limb_a = if i < len_a { a[i] } else { U256::ZERO };
            let limb_b = if i < len_b { b[i] } else { U256::ZERO };

            if limb_a != limb_b {
                return limb_a == limb_b;
            }
        }

        true
    }

    /// Compare two slices of U256 as big integers
    /// 
    /// It assumes b has no leading zeros
    pub fn lt_slices(a: &[U256], b: &[U256]) -> bool {
        let len_a = a.len();
        let len_b = b.len();

        if len_a < len_b {
            return true;
        }

        let max_len = len_a.max(len_b);
        for i in (0..max_len).rev() {
            let limb_a = if i < len_a { a[i] } else { U256::ZERO };
            let limb_b = if i < len_b { b[i] } else { U256::ZERO };

            if limb_a != limb_b {
                return limb_a < limb_b;
            }
        }

        false
    }

    /// Compare two slices of U256 as big integers without any checks
    pub fn lt_slices_unchecked(a: &[U256], b: &[U256]) -> bool {
        let len_a = a.len();
        let len_b = b.len();
        if len_a != len_b {
            return len_a < len_b;
        }
        for i in (0..len_a).rev() {
            if a[i] != b[i] {
                return a[i] < b[i];
            }
        }
        false
    }

    /// Compare two slices of U256 as big integers
    /// 
    /// It assumes b has no leading zeros
    pub fn compare_slices(a: &[U256], b: &[U256]) -> Ordering {
        let len_a = a.len();
        let len_b = b.len();

        if len_a < len_b {
            return Ordering::Less;
        }

        let max_len = len_a.max(len_b);
        for i in (0..max_len).rev() {
            let limb_a = if i < len_a { a[i] } else { U256::ZERO };
            let limb_b = if i < len_b { b[i] } else { U256::ZERO };

            match limb_a.cmp(&limb_b) {
                Ordering::Equal => continue,
                other => return other,
            }
        }

        Ordering::Equal
    }

    /// Convert a slice of U256 to a flat slice of u64
    pub fn slice_to_flat(slice: &[U256]) -> &[u64] {
        unsafe { core::slice::from_raw_parts(slice.as_ptr() as *const u64, slice.len() * 4) }
    }

    /// Reconstruct a slice of U256 from a flat slice of u64
    pub fn slice_from_flat(flat: &[u64]) -> Vec<U256> {
        assert!(flat.len() % 4 == 0, "Flat slice length must be multiple of 4");
        flat.chunks_exact(4).map(|chunk| U256([chunk[0], chunk[1], chunk[2], chunk[3]])).collect()
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
