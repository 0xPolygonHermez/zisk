//! Row index of the `DmaCompactTable` control table (see
//! `precompiles/dma/pil/dma_compact_table.pil`).
//!
//! The table enumerates, in this order, `dst_offset` (0 is a post row, 1..7 a pre row),
//! `src_offset`, `count` (1 up to 8 for a post row, 8 - dst_offset for a pre row) and four
//! variants of each combination:
//!
//! ```text
//!   0  source read from memory, no memcmp difference (memcpy, memcmp with equal bytes)
//!   1  memcmp with a positive difference
//!   2  memcmp with a negative difference
//!   3  no source read (inputcpy, memset)
//! ```

pub enum DmaCompactTable {}

impl DmaCompactTable {
    pub const VARIANT_LOAD_SRC: usize = 0;
    pub const VARIANT_POSITIVE: usize = 1;
    pub const VARIANT_NEGATIVE: usize = 2;
    pub const VARIANT_NO_SRC: usize = 3;
    const VARIANTS: usize = 4;

    /// First row of each `dst_offset` block: 8 source offsets, `max_count` counts, 4 variants.
    const OFFSETS: [usize; 8] = [0, 256, 480, 672, 832, 960, 1056, 1120];

    /// Counts a row with this destination offset can have.
    #[inline(always)]
    const fn max_count(dst_offset: usize) -> usize {
        if dst_offset == 0 {
            8
        } else {
            8 - dst_offset
        }
    }

    /// Row of the table for a pre/post row.
    #[inline(always)]
    pub fn get_row(
        dst_offset: usize,
        src_offset: usize,
        count: usize,
        load_src: bool,
        memcmp_nz: bool,
        memcmp_neg: bool,
    ) -> usize {
        debug_assert!(dst_offset < 8 && src_offset < 8);
        debug_assert!(count >= 1 && count <= Self::max_count(dst_offset));
        debug_assert!(!memcmp_neg || memcmp_nz);
        debug_assert!(load_src || !memcmp_nz);
        let variant = if !load_src {
            Self::VARIANT_NO_SRC
        } else if !memcmp_nz {
            Self::VARIANT_LOAD_SRC
        } else if memcmp_neg {
            Self::VARIANT_NEGATIVE
        } else {
            Self::VARIANT_POSITIVE
        };
        Self::OFFSETS[dst_offset]
            + (src_offset * Self::max_count(dst_offset) + (count - 1)) * Self::VARIANTS
            + variant
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_pil::DMA_COMPACT_TABLE_SIZE;

    /// The rows follow the enumeration of the PIL table exactly and cover it once.
    #[test]
    fn rows_match_the_pil_enumeration() {
        let mut expected = 0usize;
        for dst_offset in 0..8 {
            let max_count = if dst_offset == 0 { 8 } else { 8 - dst_offset };
            for src_offset in 0..8 {
                for count in 1..=max_count {
                    let rows = [
                        DmaCompactTable::get_row(dst_offset, src_offset, count, true, false, false),
                        DmaCompactTable::get_row(dst_offset, src_offset, count, true, true, false),
                        DmaCompactTable::get_row(dst_offset, src_offset, count, true, true, true),
                        DmaCompactTable::get_row(
                            dst_offset, src_offset, count, false, false, false,
                        ),
                    ];
                    for (k, row) in rows.iter().enumerate() {
                        assert_eq!(*row, expected + k, "do={dst_offset} so={src_offset} c={count}");
                    }
                    expected += 4;
                }
            }
        }
        assert_eq!(expected, DMA_COMPACT_TABLE_SIZE);
    }
}
