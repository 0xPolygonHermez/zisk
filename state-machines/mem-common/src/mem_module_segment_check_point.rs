use std::collections::HashMap;
#[cfg(feature = "debug_mem")]
use std::{
    fs::File,
    io::{BufWriter, Write},
};

use crate::MemModuleCheckPoint;
use zisk_common::ChunkId;

/// Page size for the paged-dense offsets layout.
/// The C++ side mirrors these as
// `MEM_OFFSETS_PAGE_SIZE` and `MEM_OFFSETS_PAGE_ABSENT` in
// `mem-cpp/cpp/instance_meta.hpp`. If you change a value here, change it
// there too — there is no compile-time cross-language check.
pub const MEM_OFFSETS_PAGE_SIZE_LOG2: u32 = 10;
pub const MEM_OFFSETS_PAGE_SIZE: u32 = 1 << MEM_OFFSETS_PAGE_SIZE_LOG2;
const MEM_OFFSETS_PAGE_MASK: u32 = MEM_OFFSETS_PAGE_SIZE - 1;

/// Sentinel stored in `page_starts[p]` to mark page `p` as absent
/// (i.e. all slots in the page equal `page_single_value[p]`). also defined in
// `mem-cpp/cpp/instance_meta.hpp`. If you change a value here, change it
// there too — there is no compile-time cross-language check.
pub const MEM_OFFSETS_PAGE_ABSENT: u32 = u32::MAX;

//   offsets_base_addr  — first byte address of the segment's address range
//                        (the byte address mapped to dense slot 0).
//   addr_range_slots   — ("offsets_last_addr" - offsets_base_addr)/8 + 1, dense slot count
//   num_pages          — ceil(addr_range_slots / MEM_OFFSETS_PAGE_SIZE)
//   present_count      — number of non-absent pages
//   page_starts[p]     — MEM_OFFSETS_PAGE_ABSENT iff page p is absent
//                        (uniform value = page_single_value[p]); otherwise
//                        the present-page index into `pages_dense`
//   page_single_value[p] — the value held by every slot in page p
//                          (the only value for absent pages, ignore if present)
//   pages_dense        — concatenated present-page slot data; the slice for
//                        a present page p is at
//                        pages_dense[page_starts[p] * MEM_OFFSETS_PAGE_SIZE
//                                   .. (page_starts[p]+1) * MEM_OFFSETS_PAGE_SIZE].
//                        Length = present_count * MEM_OFFSETS_PAGE_SIZE; the
//                        last partial page is padded with its carry value.
#[derive(Debug, Default, Clone)]
pub struct MemModuleSegmentCheckPoint {
    pub chunks: HashMap<ChunkId, MemModuleCheckPoint>,
    pub offsets_base_addr: u32,
    pub addr_range_slots: u32,
    pub num_pages: u32,
    pub present_count: u32,
    pub page_starts: Vec<u32>,
    pub page_single_value: Vec<u32>,
    pub pages_dense: Vec<u32>,
    pub first_chunk_id: Option<ChunkId>,
    pub is_last_segment: bool,
}

impl MemModuleSegmentCheckPoint {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Incrementally fill the paged-dense offsets layout one address at a time.
    ///
    /// `addr_w` is the qword address (byte address divided by 8) and `offset`
    /// is the value materialised at that slot. Addresses must arrive sorted:
    /// each call's `addr_w` must be `>=` the previous one.
    ///
    /// Semantics of the intermediate range: when there is a gap between the
    /// previously added address and `addr_w`, every intermediate slot takes the
    /// *same* `offset` value (the offset is a monotone running pointer, so an
    /// address with no own value carries the value of the next populated
    /// address). In other words this call paints `(previous_addr_w, addr_w]`
    /// with `offset`. The first call defines `offsets_base_addr` (slot 0).
    ///
    /// If `addr_w` repeats the previously added address, the call is ignored:
    /// the first `offset` seen for an address is the valid one (it is the row of
    /// that address's first operation) and any later offset for the same address
    /// is discarded.
    ///
    /// Pages are kept maximally compressed at every step: a page whose slots are
    /// all equal stays *absent* (`page_starts[p] == MEM_OFFSETS_PAGE_ABSENT`, the
    /// uniform value in `page_single_value[p]`, no physical storage). The first
    /// time a differing value lands in a page it is promoted to *present* and a
    /// physical page is appended to `pages_dense`. Fully-skipped pages inside a
    /// gap are uniform by construction and remain absent.
    ///
    /// This manages only the offsets layout (`offsets_base_addr`,
    /// `addr_range_slots`, `num_pages`, `present_count`, `page_starts`,
    /// `page_single_value`, `pages_dense`); `chunks`, `first_chunk_id` and
    /// `is_last_segment` are owned by other code.
    pub fn add_addr_offset(&mut self, addr_w: u32, offset: u32) {
        const PAGE: u64 = MEM_OFFSETS_PAGE_SIZE as u64;

        // First address: it defines slot 0 and the segment base byte address.
        if self.addr_range_slots == 0 {
            self.offsets_base_addr = addr_w << 3;
            self.ensure_num_pages(1);
            // Single slot in a single page => uniform => absent page.
            self.page_single_value[0] = offset;
            self.addr_range_slots = 1;
            return;
        }

        let base_w = (self.offsets_base_addr >> 3) as u64;
        debug_assert!(addr_w as u64 >= base_w, "address below segment base");
        let target_slot = addr_w as u64 - base_w;
        let prev_last = self.addr_range_slots as u64 - 1;
        debug_assert!(
            target_slot >= prev_last,
            "addresses must arrive sorted (addr_w >= previous addr_w)"
        );

        // A repeated address keeps the offset of its FIRST occurrence: `offset_at`
        // must point at the row of the first operation for an address, and the
        // consumer counts further operations by incrementing locally. So any call
        // for an already-recorded address (`target_slot <= prev_last`) is ignored.
        if target_slot <= prev_last {
            return;
        }

        // Slots painted with `offset` on this call: `(prev_last, target_slot]`.
        let lo = prev_last + 1;

        // Make sure the page metadata vectors cover the target slot's page.
        let pages_needed = (target_slot / PAGE) as u32 + 1;
        self.ensure_num_pages(pages_needed);

        // Paint `[lo, target_slot]` page by page so a multi-page gap costs
        // O(pages) rather than O(slots): fully-covered intermediate pages stay
        // absent and never allocate physical storage.
        let mut s = lo;
        while s <= target_slot {
            let page = s / PAGE;
            let page_base = page * PAGE;
            let in_lo = (s - page_base) as u32;
            let in_hi = (target_slot.min(page_base + PAGE - 1) - page_base) as u32;
            self.fill_page(page as u32, in_lo, in_hi, offset);
            s = page_base + in_hi as u64 + 1;
        }

        if target_slot + 1 > self.addr_range_slots as u64 {
            self.addr_range_slots = (target_slot + 1) as u32;
        }
    }

    /// Grow the per-page metadata vectors to cover `pages_needed` pages. New
    /// pages start absent (uniform) with a placeholder value that `fill_page`
    /// overwrites the first time the page is touched.
    fn ensure_num_pages(&mut self, pages_needed: u32) {
        if pages_needed > self.num_pages {
            self.page_starts.resize(pages_needed as usize, MEM_OFFSETS_PAGE_ABSENT);
            self.page_single_value.resize(pages_needed as usize, 0);
            self.num_pages = pages_needed;
        }
    }

    /// Set slots `[in_lo, in_hi]` of `page` to `value`, keeping the page in its
    /// most compressed form. Because addresses arrive sorted, a page is only
    /// ever extended on its high side, so the first page touched in a gap is the
    /// open page (`in_lo > 0`) and every later page is brand-new (`in_lo == 0`).
    fn fill_page(&mut self, page: u32, in_lo: u32, in_hi: u32, value: u32) {
        let p = page as usize;
        let pidx = self.page_starts[p];

        if pidx == MEM_OFFSETS_PAGE_ABSENT {
            if in_lo == 0 {
                // First time we touch this page: uniform so far, stays absent.
                self.page_single_value[p] = value;
                return;
            }
            let uniform = self.page_single_value[p];
            if value == uniform {
                // Still uniform: nothing to store, the page remains absent.
                return;
            }
            // A differing value arrived: promote the page to a present
            // (physical) page. It becomes the highest present index, so its
            // dense data is appended at the tail of `pages_dense`.
            let new_idx = self.present_count;
            self.present_count += 1;
            self.page_starts[p] = new_idx;
            // `page_single_value` mirrors the C++ layout (== first slot value).
            let base = (new_idx as usize) << MEM_OFFSETS_PAGE_SIZE_LOG2;
            debug_assert_eq!(base, self.pages_dense.len());
            // Fill the new dense page with `value`; the already-populated low
            // slots `[0, in_lo)` were all the previous uniform value.
            self.pages_dense.resize(base + MEM_OFFSETS_PAGE_SIZE as usize, value);
            for slot in 0..in_lo as usize {
                self.pages_dense[base + slot] = uniform;
            }
        } else {
            // Present page: write the dense slots directly.
            let base = (pidx as usize) << MEM_OFFSETS_PAGE_SIZE_LOG2;
            for slot in in_lo as usize..=in_hi as usize {
                self.pages_dense[base + slot] = value;
            }
        }
    }

    /// Materialised offset value at qword slot `k` (k < `addr_range_slots`).
    #[inline]
    pub fn offset_at(&self, k: u32) -> u32 {
        debug_assert!(k < self.addr_range_slots);
        let page = (k >> MEM_OFFSETS_PAGE_SIZE_LOG2) as usize;
        let in_page = (k & MEM_OFFSETS_PAGE_MASK) as usize;
        let pidx = self.page_starts[page];
        if pidx == MEM_OFFSETS_PAGE_ABSENT {
            self.page_single_value[page]
        } else {
            self.pages_dense[((pidx as usize) << MEM_OFFSETS_PAGE_SIZE_LOG2) + in_page]
        }
    }

    /// Qword-address of the largest slot `j < k` whose offset value differs
    /// from `offset_at(k)`, or `None` when no such `j` exists.
    #[inline]
    pub fn previous_change_addr_w(&self, k: u32) -> Option<u64> {
        debug_assert!(k < self.addr_range_slots);
        let cur_val = self.offset_at(k);
        let base_w = (self.offsets_base_addr as u64) >> 3;
        let mut page = k >> MEM_OFFSETS_PAGE_SIZE_LOG2;
        let mut upper = (k & MEM_OFFSETS_PAGE_MASK) as usize;
        loop {
            let pidx = self.page_starts[page as usize];
            if pidx == MEM_OFFSETS_PAGE_ABSENT {
                if self.page_single_value[page as usize] != cur_val {
                    return Some(
                        base_w
                            + (page << MEM_OFFSETS_PAGE_SIZE_LOG2) as u64
                            + (MEM_OFFSETS_PAGE_SIZE - 1) as u64,
                    );
                }
            } else {
                let start = (pidx as usize) << MEM_OFFSETS_PAGE_SIZE_LOG2;
                let dense = &self.pages_dense[start..start + MEM_OFFSETS_PAGE_SIZE as usize];
                for j in (0..upper).rev() {
                    if dense[j] != cur_val {
                        return Some(
                            base_w + (page << MEM_OFFSETS_PAGE_SIZE_LOG2) as u64 + j as u64,
                        );
                    }
                }
            }
            if page == 0 {
                return None;
            }
            page -= 1;
            upper = MEM_OFFSETS_PAGE_SIZE as usize;
        }
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_offsets_to_file(&self, file_name: &str) {
        println!("[MemDebug] saving offsets to {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let base = self.offsets_base_addr as u64;
        let mut prev_value = u32::MAX;
        for index in 0..self.addr_range_slots {
            let value = self.offset_at(index);
            if value != prev_value {
                let addr = index as u64 * 8 + base;
                if value == 0 {
                    writeln!(writer, "{:#010X} PREV_SEGMENT", addr).unwrap();
                } else {
                    writeln!(writer, "{:#010X} {}", addr, value - 1).unwrap();
                }
                prev_value = value;
            }
        }
        println!("[MemDebug] done");
    }

    pub fn to_string(&self, segment_id: usize) -> String {
        let mut result = String::new();
        for (chunk_id, checkpoint) in &self.chunks {
            result = result
                + &format!(
                    "MEM #{}@{} [0x{:08X} s:{}] [0x{:08X} C:{}] C:{}{}{}\n",
                    segment_id,
                    chunk_id,
                    checkpoint.from_addr * 8,
                    checkpoint.from_skip,
                    checkpoint.to_addr * 8,
                    checkpoint.to_count,
                    checkpoint.count,
                    if Some(*chunk_id) == self.first_chunk_id { " [first_chunk]" } else { "" },
                    if self.is_last_segment { " [last_segment]" } else { "" }
                );
        }
        result
    }
}

#[cfg(test)]
mod add_addr_offset_tests {
    use super::*;

    /// Brute-force reference: replay the same (addr_w, offset) stream into a
    /// flat dense vector following the "paint `(prev, addr_w]` with offset"
    /// contract, then compare against the compressed structure.
    fn reference_dense(base_w: u32, ops: &[(u32, u32)]) -> Vec<u32> {
        let last_w = ops.last().unwrap().0;
        let mut dense = vec![0u32; (last_w - base_w + 1) as usize];
        let mut prev_slot: i64 = -1;
        for &(addr_w, offset) in ops {
            let target = (addr_w - base_w) as i64;
            // Repeated address: keep the first offset, ignore the rest.
            if target <= prev_slot {
                continue;
            }
            for slot in (prev_slot + 1)..=target {
                dense[slot as usize] = offset;
            }
            prev_slot = target;
        }
        dense
    }

    fn check(ops: &[(u32, u32)]) {
        let mut seg = MemModuleSegmentCheckPoint::new();
        for &(addr_w, offset) in ops {
            seg.add_addr_offset(addr_w, offset);
        }

        let base_w = ops[0].0;
        let dense = reference_dense(base_w, ops);

        // Base and range.
        assert_eq!(seg.offsets_base_addr, base_w << 3);
        assert_eq!(seg.addr_range_slots as usize, dense.len());
        let expected_pages = (dense.len() as u32).div_ceil(MEM_OFFSETS_PAGE_SIZE);
        assert_eq!(seg.num_pages, expected_pages);

        // Dense storage is exactly present_count physical pages.
        assert_eq!(
            seg.pages_dense.len(),
            seg.present_count as usize * MEM_OFFSETS_PAGE_SIZE as usize
        );

        // Every slot reads back correctly.
        for (k, &want) in dense.iter().enumerate() {
            assert_eq!(seg.offset_at(k as u32), want, "offset_at({k})");
        }

        // Each page is genuinely uniform iff stored as absent.
        for p in 0..seg.num_pages as usize {
            let start = p * MEM_OFFSETS_PAGE_SIZE as usize;
            let end = ((p + 1) * MEM_OFFSETS_PAGE_SIZE as usize).min(dense.len());
            let uniform = dense[start..end].iter().all(|&v| v == dense[start]);
            let is_absent = seg.page_starts[p] == MEM_OFFSETS_PAGE_ABSENT;
            assert_eq!(is_absent, uniform, "page {p} compression mismatch");
        }

        // previous_change_addr_w must match a brute-force backward scan.
        for k in 0..dense.len() {
            let cur = dense[k];
            let expected =
                (0..k).rev().find(|&j| dense[j] != cur).map(|j| base_w as u64 + j as u64);
            assert_eq!(
                seg.previous_change_addr_w(k as u32),
                expected,
                "previous_change_addr_w({k})"
            );
        }
    }

    #[test]
    fn single_address() {
        check(&[(100, 1)]);
    }

    #[test]
    fn contiguous_same_page() {
        check(&[(10, 1), (11, 2), (12, 3), (13, 3), (14, 5)]);
    }

    #[test]
    fn gap_within_page_uniform_stays_absent() {
        // A gap painted with a constant keeps the page uniform/absent.
        check(&[(0, 7), (500, 7), (900, 7)]);
    }

    #[test]
    fn gap_promotes_page_to_present() {
        check(&[(0, 1), (5, 9), (1000, 9)]);
    }

    #[test]
    fn multi_page_gap_skips_pages() {
        // base in page 0, jump deep into page 5 -> intermediate pages absent.
        let page = MEM_OFFSETS_PAGE_SIZE;
        check(&[(0, 1), (3, 4), (5 * page + 7, 4), (5 * page + 50, 100)]);
    }

    #[test]
    fn value_change_exactly_on_page_boundary() {
        let page = MEM_OFFSETS_PAGE_SIZE;
        check(&[(0, 1), (page - 1, 1), (page, 2), (page + 10, 3)]);
    }

    #[test]
    fn repeated_address_keeps_first_offset() {
        // The same address arrives several times with different (increasing)
        // offsets; only the first occurrence's offset must survive.
        check(&[(10, 1), (20, 5), (20, 6), (20, 7), (25, 8)]);

        // Direct assertion: slot for addr 20 keeps offset 5, not 6/7.
        let mut seg = MemModuleSegmentCheckPoint::new();
        for &(a, o) in &[(10u32, 1u32), (20, 5), (20, 6), (20, 7), (25, 8)] {
            seg.add_addr_offset(a, o);
        }
        let base_w = 10;
        assert_eq!(seg.offset_at(20 - base_w), 5);
    }

    #[test]
    fn many_pages_mixed() {
        let page = MEM_OFFSETS_PAGE_SIZE;
        let mut ops = Vec::new();
        let mut offset = 1u32;
        // dense-ish region in page 0
        for i in 0..50u32 {
            ops.push((i, offset));
            offset += 1;
        }
        // big uniform gap spanning several pages, then a change
        ops.push((4 * page + 3, offset));
        offset += 1;
        ops.push((4 * page + 4, offset));
        offset += 1;
        // another multi-page jump
        ops.push((9 * page + 1000, offset));
        check(&ops);
    }
}
