//! Row layout of the `DmaCompact` AIR.
//!
//! `DmaCompact` proves a whole DMA operation with rows of three kinds (see
//! `precompiles/dma/pil/dma_compact.pil`), so one operation expands into several rows:
//!
//! ```text
//!   1 controller row  +  [1 pre row]  +  [loop rows]  +  [1 post row]
//! ```
//!
//! This module is the single place that decides that expansion. The counter (to know how many rows
//! an operation takes), the planner (to pack operations into instances) and the witness computation
//! (to fill the rows) must agree on it, so none of them derives it on its own.
//!
//! ## Loop rows
//!
//! The memory accesses that the `Mem` state machine will see are generated per operation during
//! the counting phase (see `generate_dma_mem_inputs`), before any planning, so they do not know
//! which machine will prove the operation. The loop rows of `DmaCompact` therefore reproduce the
//! accesses of the big machines exactly:
//!
//! | loop                       | accesses                                  | rows                    |
//! |----------------------------|-------------------------------------------|-------------------------|
//! | aligned (memcpy, memcmp)   | 1 source read + 1 destination op per word | 1, 2 or 4 words per row |
//! | aligned (memset, inputcpy) | 1 destination write per word              | 1, 2 or 4 words per row |
//! | unaligned (memcpy, memcmp) | words + 1 source reads, 1 op per word     | 1 word per row + 1 tail |
//!
//! An unaligned loop (the source offset after the pre phase is not 0) takes the high half of the
//! window of each row from the next row, as `DmaUnaligned` does, so every source word is read
//! once and the sequence needs an extra read only row at the end.
//!
//! Every DMA operation can be proved here, including the "direct" ones that the big machines
//! prove without a controller (`DmaCompact` always has a controller row). The only limit is the
//! instance size: an operation is never split, so one whose rows do not fit in an instance cannot
//! be routed here at all (see `DmaStrategy`).

use precompiles_helpers::DmaInfo;
use zisk_core::zisk_ops::ZiskOp;

/// Words that one aligned loop row can copy, compare or set: `sel_w2` and `sel_w34` give 1, 2 or 4.
pub const DMA_COMPACT_MAX_WORDS_X_ROW: usize = 4;

/// Words that one aligned loop row of an inputcpy can copy. The values of words 3 and 4 live in
/// columns that are not byte range checked, and the input of an inputcpy is free, so only the words
/// that fit in the (range checked) window bytes are allowed: see `sel_inputcpy * sel_w34 === 0`.
pub const DMA_COMPACT_MAX_WORDS_X_ROW_INPUTCPY: usize = 2;

/// Rows that an operation needs in a `DmaCompact` instance, split per row kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DmaCompactLayout {
    /// Controller rows: always 1.
    pub ctrl: usize,
    /// Pre partial write rows: 1 when the destination is not 8-aligned and there are bytes for it.
    pub pre: usize,
    /// Post partial write rows: 1 when the tail does not complete a 64-bit word.
    pub post: usize,
    /// Loop rows, already taking into account how many words each one does and the extra read only
    /// row of an unaligned loop.
    pub loop_rows: usize,
    /// 64-bit words that the loop part copies (sum of the words of every loop row).
    pub loop_words: usize,
    /// Window offset of the loop (the source offset after the pre phase): 0 for an aligned loop.
    pub delta: u8,
}

impl DmaCompactLayout {
    /// Whether the operation reads its source from memory (memcpy, memcmp).
    #[inline(always)]
    pub fn has_src(op: u8) -> bool {
        matches!(
            op,
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY | ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP
        )
    }

    /// Whether `op` is one of the DMA operations `DmaCompact` knows.
    #[inline(always)]
    pub fn is_dma_op(op: u8) -> bool {
        Self::has_src(op) || op == ZiskOp::DMA_INPUTCPY || op == ZiskOp::DMA_XMEMSET
    }

    /// Maximum number of words that an aligned loop row can do for a given operation.
    #[inline(always)]
    pub fn max_words_x_row(op: u8) -> usize {
        if op == ZiskOp::DMA_INPUTCPY {
            DMA_COMPACT_MAX_WORDS_X_ROW_INPUTCPY
        } else {
            DMA_COMPACT_MAX_WORDS_X_ROW
        }
    }

    /// Words that the aligned loop row starting with `remaining_words` left must do: the greedy
    /// policy, 4 while at least 4 are left, then 2, then 1 (and 2, 1 for an inputcpy).
    ///
    /// Any policy is sound (each row commits to its own count and hands over the exact remainder),
    /// this one just minimizes rows.
    #[inline(always)]
    pub fn loop_row_words(op: u8, remaining_words: usize) -> usize {
        debug_assert!(remaining_words > 0);
        let max = Self::max_words_x_row(op);
        if remaining_words >= 4 && max >= 4 {
            4
        } else if remaining_words >= 2 {
            2
        } else {
            1
        }
    }

    /// Row layout of an operation.
    pub fn of(op: u8, encoded: u64) -> Self {
        debug_assert!(Self::is_dma_op(op), "DmaCompactLayout::of unsupported op 0x{op:02X}");

        let loop_words = DmaInfo::get_loop_count(encoded);
        let delta = if Self::has_src(op) && loop_words > 0 {
            DmaInfo::get_loop_src_offset(encoded)
        } else {
            0
        };

        let loop_rows = if loop_words == 0 {
            0
        } else if delta != 0 {
            // one word per row plus the read only tail row
            loop_words + 1
        } else if Self::max_words_x_row(op) >= 4 {
            // 4 words per row, and the remainder 1..3 words takes one row of 2 plus one of 1
            let full = loop_words / 4;
            let rest = loop_words % 4;
            full + match rest {
                0 => 0,
                1 => 1, // 1
                2 => 1, // 2
                _ => 2, // 3 = 2 + 1
            }
        } else {
            // 2 words per row, and an odd remainder takes one more row
            loop_words.div_ceil(2)
        };

        Self {
            ctrl: 1,
            pre: usize::from(DmaInfo::get_pre_count(encoded) > 0),
            post: usize::from(DmaInfo::get_post_count(encoded) > 0),
            loop_rows,
            loop_words,
            delta,
        }
    }

    /// Whether the loop is the unaligned one (one word per row, window from the next row).
    #[inline(always)]
    pub fn unaligned(&self) -> bool {
        self.delta != 0
    }

    /// Total rows the operation occupies in an instance.
    #[inline(always)]
    pub fn rows(&self) -> usize {
        self.ctrl + self.pre + self.post + self.loop_rows
    }

    /// Rows an operation occupies in an instance.
    #[inline(always)]
    pub fn rows_of(op: u8, encoded: u64) -> usize {
        Self::of(op, encoded).rows()
    }

    /// The sequence of words per loop row, in chain order (the read only tail row of an unaligned
    /// loop is not included, it does 0 words). It always sums to `loop_words`.
    pub fn loop_row_words_seq(op: u8, encoded: u64) -> Vec<usize> {
        let layout = Self::of(op, encoded);
        if layout.unaligned() {
            return vec![1; layout.loop_words];
        }
        let mut remaining = layout.loop_words;
        let mut out = Vec::new();
        while remaining > 0 {
            let words = Self::loop_row_words(op, remaining);
            out.push(words);
            remaining -= words;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every operation is covered exactly: the pre bytes, the loop words and the post bytes add up
    /// to the requested count, and the per row word counts add up to the loop words.
    #[test]
    fn layout_covers_the_whole_operation() {
        const BASE: u64 = 0xA000_1000;
        for count in (1usize..80).chain([255, 256, 257, 1000, 4096]) {
            for dst_offset in 0u64..8 {
                for src_offset in 0u64..8 {
                    let dst = BASE + 0x800 + dst_offset;
                    let src = BASE + src_offset;
                    let encoded = DmaInfo::encode_memcpy(dst, src, count);
                    let op = ZiskOp::DMA_XMEMCPY;

                    let layout = DmaCompactLayout::of(op, encoded);
                    let bytes = DmaInfo::get_pre_count(encoded)
                        + layout.loop_words * 8
                        + DmaInfo::get_post_count(encoded);
                    assert_eq!(bytes, count, "count mismatch c={count} do={dst_offset}");

                    let seq = DmaCompactLayout::loop_row_words_seq(op, encoded);
                    assert_eq!(seq.iter().sum::<usize>(), layout.loop_words);
                    if layout.loop_words == 0 {
                        assert_eq!(layout.loop_rows, 0);
                        assert_eq!(layout.delta, 0);
                        continue;
                    }
                    if dst_offset != src_offset {
                        assert!(layout.unaligned(), "unaligned loop expected c={count}");
                        assert_eq!(seq.len() + 1, layout.loop_rows, "tail row c={count}");
                        assert!(seq.iter().all(|&w| w == 1));
                    } else {
                        assert!(!layout.unaligned());
                        assert_eq!(seq.len(), layout.loop_rows, "rows mismatch c={count}");
                        assert!(seq.iter().all(|&w| w == 1 || w == 2 || w == 4));
                    }
                }
            }
        }
    }

    /// memset and inputcpy never have an unaligned loop; inputcpy never uses more than 2 words
    /// per row.
    #[test]
    fn no_source_operations() {
        const BASE: u64 = 0xA000_1000;
        for count in (1usize..80).chain([1024]) {
            for dst_offset in 0u64..8 {
                let dst = BASE + 0x800 + dst_offset;

                let encoded = DmaInfo::encode_memset(dst, count, 0xAB);
                let layout = DmaCompactLayout::of(ZiskOp::DMA_XMEMSET, encoded);
                assert!(!layout.unaligned());
                let seq = DmaCompactLayout::loop_row_words_seq(ZiskOp::DMA_XMEMSET, encoded);
                assert!(seq.iter().all(|&w| w == 1 || w == 2 || w == 4));
                assert_eq!(seq.len(), layout.loop_rows);

                let encoded = DmaInfo::encode_inputcpy(dst, count);
                let layout = DmaCompactLayout::of(ZiskOp::DMA_INPUTCPY, encoded);
                assert!(!layout.unaligned());
                let seq = DmaCompactLayout::loop_row_words_seq(ZiskOp::DMA_INPUTCPY, encoded);
                assert!(
                    seq.iter().all(|&w| w == 1 || w == 2),
                    "inputcpy limited to 2 words: {seq:?}"
                );
                assert_eq!(seq.iter().sum::<usize>(), DmaInfo::get_loop_count(encoded));
                assert_eq!(seq.len(), layout.loop_rows);
            }
        }
    }

    /// A tiny operation that fits in the pre row alone takes 2 rows (controller + pre).
    #[test]
    fn small_operations() {
        const BASE: u64 = 0xA000_1000;
        // 3 bytes at offset 4: only a pre row
        let encoded = DmaInfo::encode_memcpy(BASE + 0x804, BASE + 4, 3);
        let layout = DmaCompactLayout::of(ZiskOp::DMA_XMEMCPY, encoded);
        assert_eq!((layout.pre, layout.loop_rows, layout.post), (1, 0, 0));
        assert_eq!(layout.rows(), 2);

        // one full aligned word: controller + one loop row (a "direct" operation)
        let encoded = DmaInfo::encode_memcpy(BASE + 0x800, BASE, 8);
        assert!(DmaInfo::is_direct(encoded));
        let layout = DmaCompactLayout::of(ZiskOp::DMA_XMEMCPY, encoded);
        assert_eq!((layout.pre, layout.loop_rows, layout.post), (0, 1, 0));
        assert_eq!(layout.rows(), 2);

        // 32 aligned bytes: a single 4-word loop row
        let encoded = DmaInfo::encode_memcpy(BASE + 0x800, BASE, 32);
        let layout = DmaCompactLayout::of(ZiskOp::DMA_XMEMCPY, encoded);
        assert_eq!((layout.pre, layout.loop_rows, layout.post, layout.loop_words), (0, 1, 0, 4));

        // 3 unaligned words: controller + 3 loop rows + the tail row
        let encoded = DmaInfo::encode_memcpy(BASE + 0x800, BASE + 3, 24);
        let layout = DmaCompactLayout::of(ZiskOp::DMA_XMEMCPY, encoded);
        assert_eq!((layout.pre, layout.loop_rows, layout.post, layout.delta), (0, 4, 0, 3));
        assert_eq!(layout.rows(), 5);
    }
}
