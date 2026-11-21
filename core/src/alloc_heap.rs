//! Heap of the allocator precompile (`precompiles/allocator`), as the executor has to manage it.
//!
//! The precompile only validates the operations, the executor decides. This is that decision for
//! the two variants of the precompile, under one structure:
//!
//! - `Allocator` ([`AllocPolicy::Canonical`]): there is nothing to decide. The block taken is the
//!   best fit (the smallest free block that fits, the lowest address among equals) and a released
//!   block is joined with its free neighbours in the row of the `dealloc`. One row per operation.
//! - `AllocatorFlex`: the prover decides which block is taken and when free blocks are joined, and
//!   every join is a row of its own. Two policies are provided, both best fit:
//!   [`AllocPolicy::FlexEager`] joins as soon as it can (the same heap as `Canonical`) and
//!   [`AllocPolicy::FlexLazy`] only when an `alloc` finds no block that fits.
//!
//! In every policy the rest of a taken block goes back to the free blocks, a released block is
//! dirty, and blocks of different areas are never joined.
//!
//! PROTOTYPE: only the plain emulation uses it. There is no witness generation.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;

/// Total columns of `Allocator` (see `precompiles/allocator/README.md`), the cost of one row.
pub const ALLOC_ROW_COST: u64 = 156;
/// Total columns of `AllocatorFlex`, the cost of one row.
pub const ALLOC_FLEX_ROW_COST: u64 = 55;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AllocPolicy {
    /// `Allocator`: best fit, joins in the row of the dealloc.
    #[default]
    Canonical,
    /// `AllocatorFlex`: best fit, a released block is joined right away, one row per join.
    FlexEager,
    /// `AllocatorFlex`: best fit, free blocks are only joined when no block fits.
    FlexLazy,
}

impl AllocPolicy {
    pub const ALL: [AllocPolicy; 3] = [Self::Canonical, Self::FlexEager, Self::FlexLazy];

    pub const fn name(&self) -> &'static str {
        match self {
            Self::Canonical => "Allocator",
            Self::FlexEager => "AllocatorFlex eager",
            Self::FlexLazy => "AllocatorFlex lazy",
        }
    }

    /// Cost of one row of the air that proves this policy.
    pub const fn row_cost(&self) -> u64 {
        match self {
            Self::Canonical => ALLOC_ROW_COST,
            Self::FlexEager | Self::FlexLazy => ALLOC_FLEX_ROW_COST,
        }
    }
}

impl FromStr for AllocPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "canonical" => Ok(Self::Canonical),
            "flex-eager" => Ok(Self::FlexEager),
            "flex-lazy" => Ok(Self::FlexLazy),
            _ => Err(format!("unknown policy '{s}', expected canonical, flex-eager or flex-lazy")),
        }
    }
}

/// Why an operation cannot be served. In the zkVM such an execution has no proof.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocError {
    /// The size is zero or not a multiple of 8.
    InvalidSize(u64),
    /// The address of an area is not a multiple of 8.
    InvalidAddr(u64),
    /// No free block fits.
    OutOfMemory(u64),
    /// The address is not the start of a block in use.
    NotAllocated(u64),
    /// The area overlaps a block the heap already manages.
    Overlap(u64),
}

impl std::fmt::Display for AllocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSize(size) => write!(f, "size {size} is not a non-zero multiple of 8"),
            Self::InvalidAddr(addr) => write!(f, "address 0x{addr:x} is not a multiple of 8"),
            Self::OutOfMemory(size) => write!(f, "no free block of {size} bytes"),
            Self::NotAllocated(addr) => write!(f, "0x{addr:x} is not the start of a block in use"),
            Self::Overlap(addr) => write!(f, "the area at 0x{addr:x} overlaps the heap"),
        }
    }
}

/// What an `alloc` did, which is what its cost depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AllocOutcome {
    pub addr: u64,
    /// The block had been used before: a zeroed allocation has to clear it.
    pub dirty: bool,
    /// Join rows the operation needed (only `AllocatorFlex`).
    pub joins: u64,
}

#[derive(Debug, Default)]
pub struct AllocHeap {
    policy: AllocPolicy,
    /// Free blocks by address: `addr -> (size, dirty)`.
    free: BTreeMap<u64, (u64, bool)>,
    /// Free blocks by `(size, addr)`, the order of the free list of `Allocator`.
    free_by_size: BTreeSet<(u64, u64)>,
    /// Blocks in use: `addr -> size`.
    used: HashMap<u64, u64>,
    /// First address of each area, which a join never crosses.
    area_starts: BTreeSet<u64>,
    /// Rows of main operations: create, alloc, calloc and dealloc.
    pub op_rows: u64,
    /// Rows of joins (only `AllocatorFlex`).
    pub join_rows: u64,
    /// Bytes in use now, and the most there has been.
    pub used_bytes: u64,
    pub peak_used_bytes: u64,
    pub peak_used_blocks: usize,
    /// Highest end address ever handed out, i.e. how far the heap had to reach.
    pub high_water: u64,
}

impl AllocHeap {
    pub fn new(policy: AllocPolicy) -> Self {
        Self { policy, ..Default::default() }
    }

    pub fn policy(&self) -> AllocPolicy {
        self.policy
    }

    /// Changes the policy. Only before the first operation: the policies do not share the state
    /// of the free blocks.
    pub fn set_policy(&mut self, policy: AllocPolicy) {
        assert!(self.op_rows == 0, "the allocator policy cannot change once the heap is in use");
        self.policy = policy;
    }

    /// Rows of the air: main operations plus joins.
    pub fn rows(&self) -> u64 {
        self.op_rows + self.join_rows
    }

    pub fn used_blocks(&self) -> usize {
        self.used.len()
    }

    pub fn free_blocks(&self) -> usize {
        self.free.len()
    }

    /// Size of the block in use that starts at `addr`.
    pub fn used_size(&self, addr: u64) -> Option<u64> {
        self.used.get(&addr).copied()
    }

    fn insert_free(&mut self, addr: u64, size: u64, dirty: bool) {
        self.free.insert(addr, (size, dirty));
        self.free_by_size.insert((size, addr));
    }

    fn remove_free(&mut self, addr: u64) -> (u64, bool) {
        let (size, dirty) = self.free.remove(&addr).expect("free block");
        self.free_by_size.remove(&(size, addr));
        (size, dirty)
    }

    /// `create_clean` / `create_dirty`: registers the area `[addr, addr + size)`.
    pub fn create(&mut self, addr: u64, size: u64, dirty: bool) -> Result<(), AllocError> {
        if addr % 8 != 0 {
            return Err(AllocError::InvalidAddr(addr));
        }
        if size == 0 || size % 8 != 0 {
            return Err(AllocError::InvalidSize(size));
        }
        // The area has to be disjoint from everything the heap manages. Free blocks and blocks in
        // use tile the areas, so it is enough to look at the areas: an area ends where its last
        // block ends.
        let end = addr + size;
        let below = self.free.range(..end).next_back().map(|(a, b)| a + b.0).unwrap_or(0);
        if below > addr || self.used.iter().any(|(a, s)| *a < end && a + s > addr) {
            return Err(AllocError::Overlap(addr));
        }
        self.area_starts.insert(addr);
        self.insert_free(addr, size, dirty);
        self.op_rows += 1;
        Ok(())
    }

    /// `alloc` / `calloc`: takes `size` bytes from the best fit.
    pub fn alloc(&mut self, size: u64) -> Result<AllocOutcome, AllocError> {
        if size == 0 || size % 8 != 0 {
            return Err(AllocError::InvalidSize(size));
        }
        let mut joins = 0;
        let mut found = self.best_fit(size);
        if found.is_none() && self.policy == AllocPolicy::FlexLazy {
            joins = self.join_all();
            found = self.best_fit(size);
        }
        let Some(addr) = found else {
            self.join_rows += joins;
            return Err(AllocError::OutOfMemory(size));
        };
        let (block_size, dirty) = self.remove_free(addr);
        if block_size > size {
            self.insert_free(addr + size, block_size - size, dirty);
        }
        self.used.insert(addr, size);
        self.op_rows += 1;
        self.join_rows += joins;
        self.used_bytes += size;
        self.peak_used_bytes = self.peak_used_bytes.max(self.used_bytes);
        self.peak_used_blocks = self.peak_used_blocks.max(self.used.len());
        self.high_water = self.high_water.max(addr + size);
        Ok(AllocOutcome { addr, dirty, joins })
    }

    /// `dealloc`: releases the block that starts at `addr`. Returns the join rows it needed.
    pub fn dealloc(&mut self, addr: u64) -> Result<u64, AllocError> {
        let Some(size) = self.used.remove(&addr) else {
            return Err(AllocError::NotAllocated(addr));
        };
        self.used_bytes -= size;
        self.op_rows += 1;
        let (mut addr, mut size) = (addr, size);
        let mut joined = 0;
        if self.policy != AllocPolicy::FlexLazy {
            let above = addr + size;
            if !self.area_starts.contains(&above) && self.free.contains_key(&above) {
                size += self.remove_free(above).0;
                joined += 1;
            }
            if !self.area_starts.contains(&addr) {
                let below = self.free.range(..addr).next_back().map(|(a, b)| (*a, b.0));
                if let Some((below_addr, below_size)) = below {
                    if below_addr + below_size == addr {
                        self.remove_free(below_addr);
                        addr = below_addr;
                        size += below_size;
                        joined += 1;
                    }
                }
            }
        }
        self.insert_free(addr, size, true);
        // In `Allocator` the joins are part of the row of the dealloc.
        let joins = if self.policy == AllocPolicy::Canonical { 0 } else { joined };
        self.join_rows += joins;
        Ok(joins)
    }

    /// The smallest free block that fits, the lowest address among equals.
    fn best_fit(&self, size: u64) -> Option<u64> {
        self.free_by_size.range((size, 0)..).next().map(|&(_, addr)| addr)
    }

    /// Joins every pair of adjacent free blocks of the same area. Returns the number of joins.
    fn join_all(&mut self) -> u64 {
        let blocks: Vec<(u64, u64, bool)> = self.free.iter().map(|(a, b)| (*a, b.0, b.1)).collect();
        self.free.clear();
        self.free_by_size.clear();
        let mut joins = 0;
        let mut current: Option<(u64, u64, bool)> = None;
        for (addr, size, dirty) in blocks {
            current = match current {
                Some((c_addr, c_size, c_dirty))
                    if c_addr + c_size == addr && !self.area_starts.contains(&addr) =>
                {
                    joins += 1;
                    Some((c_addr, c_size + size, c_dirty || dirty))
                }
                Some((c_addr, c_size, c_dirty)) => {
                    self.insert_free(c_addr, c_size, c_dirty);
                    Some((addr, size, dirty))
                }
                None => Some((addr, size, dirty)),
            };
        }
        if let Some((addr, size, dirty)) = current {
            self.insert_free(addr, size, dirty);
        }
        joins
    }

    /// Whether the block in use at `addr` could grow to `new_size` without moving: the block
    /// right above it is free, of the same area, and big enough.
    pub fn can_grow(&self, addr: u64, new_size: u64) -> bool {
        let Some(size) = self.used_size(addr) else {
            return false;
        };
        let above = addr + size;
        !self.area_starts.contains(&above)
            && self.free.get(&above).is_some_and(|b| size + b.0 >= new_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heap(policy: AllocPolicy, size: u64) -> AllocHeap {
        let mut heap = AllocHeap::new(policy);
        heap.create(0x1000, size, false).unwrap();
        heap
    }

    #[test]
    fn best_fit_takes_the_smallest_block_and_returns_the_rest() {
        let mut heap = heap(AllocPolicy::Canonical, 1 << 20);
        let a = heap.alloc(128).unwrap().addr;
        let _guard1 = heap.alloc(8).unwrap();
        let b = heap.alloc(32).unwrap().addr;
        let _guard2 = heap.alloc(8).unwrap();
        heap.dealloc(a).unwrap();
        heap.dealloc(b).unwrap();
        // the hole of 32 fits better than the one of 128; it is dirty, it was used
        assert_eq!(heap.alloc(24), Ok(AllocOutcome { addr: b, dirty: true, joins: 0 }));
        // the 8 bytes left of b are free again
        assert_eq!(heap.alloc(8).unwrap().addr, b + 24);
        assert_eq!(heap.rows(), 1 + 4 + 2 + 2);
    }

    #[test]
    fn a_fresh_block_is_clean_and_a_released_one_is_dirty() {
        let mut heap = heap(AllocPolicy::Canonical, 1 << 20);
        let a = heap.alloc(64).unwrap();
        assert!(!a.dirty);
        let _b = heap.alloc(64).unwrap();
        heap.dealloc(a.addr).unwrap();
        assert_eq!(heap.alloc(64), Ok(AllocOutcome { addr: a.addr, dirty: true, joins: 0 }));
    }

    #[test]
    fn the_policies_differ_in_the_join_rows() {
        for (policy, joins_of_dealloc, free_blocks) in [
            (AllocPolicy::Canonical, 0, 1),
            (AllocPolicy::FlexEager, 2, 1),
            (AllocPolicy::FlexLazy, 0, 3),
        ] {
            let mut heap = heap(policy, 1 << 12);
            let a = heap.alloc(64).unwrap().addr;
            let b = heap.alloc(64).unwrap().addr;
            assert_eq!(heap.dealloc(a), Ok(0));
            // b has a free block on each side
            assert_eq!(heap.dealloc(b), Ok(joins_of_dealloc), "{policy:?}");
            assert_eq!(heap.free_blocks(), free_blocks, "{policy:?}");
            assert_eq!(heap.used_bytes, 0);
        }
    }

    #[test]
    fn lazy_joins_only_when_nothing_fits() {
        let mut heap = heap(AllocPolicy::FlexLazy, 64);
        let blocks: Vec<u64> = (0..8).map(|_| heap.alloc(8).unwrap().addr).collect();
        for &block in &blocks {
            heap.dealloc(block).unwrap();
        }
        assert_eq!(heap.join_rows, 0);
        assert_eq!(heap.alloc(64), Ok(AllocOutcome { addr: 0x1000, dirty: true, joins: 7 }));
        assert_eq!(heap.alloc(8), Err(AllocError::OutOfMemory(8)));
    }

    #[test]
    fn blocks_of_different_areas_are_never_joined() {
        for policy in AllocPolicy::ALL {
            let mut heap = heap(policy, 64);
            // a second area right after the first one
            heap.create(0x1040, 64, true).unwrap();
            let a = heap.alloc(64).unwrap().addr;
            let b = heap.alloc(64).unwrap().addr;
            heap.dealloc(a).unwrap();
            heap.dealloc(b).unwrap();
            assert_eq!(heap.free_blocks(), 2, "{policy:?}");
            assert_eq!(heap.alloc(72), Err(AllocError::OutOfMemory(72)), "{policy:?}");
            assert!(!heap.can_grow(a, 72));
        }
    }

    #[test]
    fn invalid_operations_are_rejected() {
        let mut heap = heap(AllocPolicy::Canonical, 1 << 12);
        assert_eq!(heap.alloc(0), Err(AllocError::InvalidSize(0)));
        assert_eq!(heap.alloc(12), Err(AllocError::InvalidSize(12)));
        assert_eq!(heap.create(0x1004, 64, false), Err(AllocError::InvalidAddr(0x1004)));
        assert_eq!(heap.create(0x1800, 64, false), Err(AllocError::Overlap(0x1800)));
        let a = heap.alloc(64).unwrap().addr;
        assert_eq!(heap.create(a, 64, false), Err(AllocError::Overlap(a)));
        // only the start of a block in use can be released, and only once
        assert_eq!(heap.dealloc(a + 8), Err(AllocError::NotAllocated(a + 8)));
        assert_eq!(heap.dealloc(a), Ok(0));
        assert_eq!(heap.dealloc(a), Err(AllocError::NotAllocated(a)));
    }

    #[test]
    fn a_block_can_grow_in_place_when_the_one_above_is_free_and_big_enough() {
        let mut heap = heap(AllocPolicy::Canonical, 256);
        let a = heap.alloc(64).unwrap().addr;
        assert!(heap.can_grow(a, 256));
        assert!(!heap.can_grow(a, 264));
        let _b = heap.alloc(8).unwrap();
        assert!(!heap.can_grow(a, 72));
    }
}
