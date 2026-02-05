use std::{
    cmp::Ordering,
    fmt::{self, Debug, Display},
};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct U256([u64; 4]); // little-endian: 4 × 64 = 256 bits

impl U256 {
    pub const ZERO: Self = U256([0, 0, 0, 0]);
    pub const ONE: Self = U256([1, 0, 0, 0]);
    pub const TWO: Self = U256([2, 0, 0, 0]);
    pub const MAX: Self = U256([u64::MAX, u64::MAX, u64::MAX, u64::MAX]);

    #[inline(always)]
    pub const fn from_u64s(a: &[u64; 4]) -> Self {
        U256(*a)
    }

    #[inline(always)]
    pub const fn from_u64(a: u64) -> Self {
        U256([a, 0, 0, 0])
    }

    #[inline(always)]
    pub fn as_limbs(&self) -> &[u64; 4] {
        &self.0
    }

    #[inline(always)]
    pub fn as_limbs_mut(&mut self) -> &mut [u64; 4] {
        &mut self.0
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
    }

    #[inline]
    pub fn is_one(&self) -> bool {
        self.0[0] == 1 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
    }

    #[inline]
    pub fn lt(&self, other: &Self) -> bool {
        for i in (0..4).rev() {
            if self.0[i] != other.0[i] {
                return self.0[i] < other.0[i];
            }
        }
        false
    }

    #[inline]
    pub fn gt(&self, other: &Self) -> bool {
        for i in (0..4).rev() {
            if self.0[i] != other.0[i] {
                return self.0[i] > other.0[i];
            }
        }
        false
    }

    #[inline]
    pub fn compare(&self, other: &Self) -> Ordering {
        for i in (0..4).rev() {
            if self.0[i] < other.0[i] {
                return Ordering::Less;
            } else if self.0[i] > other.0[i] {
                return Ordering::Greater;
            }
        }
        Ordering::Equal
    }

    pub fn eq_slices(a: &[Self], b: &[Self]) -> bool {
        // TODO: Do with hint and instructions?

        let len_a = a.len();
        let len_b = b.len();
        if len_a != len_b {
            return false;
        }

        for i in 0..len_a {
            if !a[i].eq(&b[i]) {
                return false;
            }
        }

        true
    }

    pub fn lt_slices(a: &[Self], b: &[Self]) -> bool {
        // TODO: Do with hint and instructions?

        let len_a = a.len();
        let len_b = b.len();
        if len_a != len_b {
            return len_a < len_b;
        }

        for i in (0..len_a).rev() {
            if !a[i].eq(&b[i]) {
                return a[i].lt(&b[i]);
            }
        }

        false
    }

    pub fn compare_slices(a: &[U256], b: &[U256]) -> Ordering {
        // TODO: Do with hint and instructions?

        let len_a = a.len();
        let len_b = b.len();

        if len_a != len_b {
            return len_a.cmp(&len_b);
        }

        for i in (0..len_a).rev() {
            match a[i].compare(&b[i]) {
                Ordering::Equal => continue,
                other => return other,
            }
        }

        Ordering::Equal
    }

    #[inline(always)]
    pub fn slice_to_flat(slice: &[U256]) -> &[u64] {
        // Safe because U256 is #[repr(transparent)] over [u64; 4]
        unsafe { core::slice::from_raw_parts(slice.as_ptr() as *const u64, slice.len() * 4) }
    }

    #[inline(always)]
    pub fn flat_to_slice(flat: &[u64]) -> &[U256] {
        debug_assert_eq!(flat.len() % 4, 0, "Flat slice length must be multiple of 4");
        // Safe because U256 is #[repr(transparent)] over [u64; 4]
        unsafe { core::slice::from_raw_parts(flat.as_ptr() as *const U256, flat.len() / 4) }
    }
}

impl Debug for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}{:016x}{:016x}{:016x}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}{:016x}{:016x}{:016x}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl PartialEq for U256 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0[3] == other.0[3]
            && self.0[2] == other.0[2]
            && self.0[1] == other.0[1]
            && self.0[0] == other.0[0]
    }
}

pub struct ShortScratch {
    // For rem_short verification
    pub quo: [u64; 8],    // quotient
    pub rem: [u64; 4],    // remainder
    pub q_b: [U256; 2],   // q * b
    pub q_b_r: [U256; 2], // q * b + r
}

impl ShortScratch {
    #[inline(always)]
    pub fn new() -> Self {
        Self { quo: [0u64; 8], rem: [0u64; 4], q_b: [U256::ZERO; 2], q_b_r: [U256::ZERO; 2] }
    }
}

impl Default for ShortScratch {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RemLongScratch {
    pub quo: Vec<u64>,    // quotient
    pub rem: Vec<u64>,    // remainder
    pub q_b: Vec<U256>,   // q * b
    pub q_b_r: Vec<U256>, // q * b + r
}

impl RemLongScratch {
    pub fn new(len_m: usize) -> Self {
        let max_quo = (2 * len_m) * 4;
        let max_rem = len_m * 4;
        let max_prod = 2 * len_m;
        Self {
            quo: vec![0u64; max_quo],
            rem: vec![0u64; max_rem],
            q_b: vec![U256::ZERO; max_prod],
            q_b_r: vec![U256::ZERO; max_prod],
        }
    }
}

pub struct LongScratch {
    // For rem_long verification
    pub rem: RemLongScratch,
    // For mul_long / square_long
    pub mul: Vec<U256>, // result of mul or square
}

impl LongScratch {
    pub fn new(len_m: usize) -> Self {
        let max_mul = 2 * len_m;
        Self { rem: RemLongScratch::new(len_m), mul: vec![U256::ZERO; max_mul] }
    }
}
