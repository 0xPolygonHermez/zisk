//! Row values of the `DmaCompact` AIR.
//!
//! The witness computation builds rows here and then copies them into the trace, so the PIL
//! formulas live in one place and can be unit tested without a prover. Columns that hold small
//! values keep their natural type (the trace setters convert them), and the ones that carry a bus
//! payload are field elements, because the memcmp byte factor is signed.
//!
//! Every column that the AIR defines with an UNGATED constraint is derived in [`DmaCompactRow::
//! finish`], so a row built with [`DmaCompactRow::ctrl`], [`DmaCompactRow::pre_post`],
//! [`DmaCompactRow::loop_sequence`] or [`DmaCompactRow::padding`] satisfies them by construction,
//! whatever its kind.

use fields::PrimeField64;
use precompiles_helpers::DmaInfo;
use zisk_core::zisk_ops::ZiskOp;

use crate::DmaCompactLayout;

/// Operation code of the internal 64-bit compare, `OP_DMA_XMEMEQ` in `pil/operations.pil`. It is
/// not a `ZiskOp` entry because Main never issues it.
pub const OP_DMA_XMEMEQ: u64 = 0xDA;
/// Pre operation codes of DMA_BUS, `OP_DMA_PRE_*` in `precompiles/dma/pil/dma.pil`.
pub const OP_DMA_PRE_XMEMCPY: u64 = 1;
pub const OP_DMA_PRE_INPUTCPY: u64 = 2;
pub const OP_DMA_PRE_XMEMCMP: u64 = 4;
pub const OP_DMA_PRE_XMEMSET: u64 = 5;
/// Added to a pre operation code to get the post one.
pub const OP_DMA_POST_OFFSET: u64 = 10;

/// One row of the `DmaCompact` trace. Field names match the AIR columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmaCompactRow<F: PrimeField64> {
    pub is_ctrl: bool,
    pub is_pp: bool,
    pub is_loop: bool,
    pub sel_memcpy: bool,
    pub sel_memcmp: bool,
    pub sel_inputcpy: bool,
    pub sel_memset: bool,
    pub rb: [u8; 16],
    pub pb: [u8; 8],
    pub abs_diff: u8,
    pub sb: [bool; 8],
    pub sr: [bool; 7],
    /// loop: `seq_end`; every other row kind (controller, pre/post, padding): 1.
    pub pp_flag_a: bool,
    pub borrow: bool,
    pub read_lo: bool,
    pub read_hi: bool,
    pub memcmp_nz: bool,
    pub memcmp_neg: bool,
    pub load_count: bool,
    pub main_step: u64,
    pub dst64: u64,
    pub src64: u64,
    pub dst_offset: u8,
    pub src_offset: u8,
    pub fill_byte: u8,
    pub cnt_small: u8,
    pub count: u64,
    pub rom_l_count64: u64,
    pub pp_cmp: bool,
    pub sel_w2: bool,
    pub sel_w34: bool,
    pub rd_34: bool,
    pub loop_end: bool,
    pub wr_loop: bool,
    pub wv: [u64; 2],
    pub v3: [u64; 2],
    pub v4: [u64; 2],
    /// ctrl: result of the pre phase; pre/post: the memcmp result of the row.
    pub rv: [u64; 2],
    /// ctrl: result of the post phase; pre/post: the memcmp byte factor, which is signed.
    pub pw: [F; 2],
    pub op_col: u64,
    pub b0_col: u64,
    pub c0_col: u64,
    pub ext_col: u64,
    pub xa_col: u64,
    pub b0_loop: u64,
    pub ext_loop: u64,
    pub x_count: u64,
    /// Not a column: `count_bus - count`, needed to rebuild `count_bus` in [`Self::finish`].
    count_diff: u64,
    /// Not a column: bytes of the loop part, only meaningful on a controller row.
    loop_count_bytes: u64,
}

impl<F: PrimeField64> Default for DmaCompactRow<F> {
    fn default() -> Self {
        Self {
            is_ctrl: false,
            is_pp: false,
            is_loop: false,
            sel_memcpy: false,
            sel_memcmp: false,
            sel_inputcpy: false,
            sel_memset: false,
            rb: [0; 16],
            pb: [0; 8],
            abs_diff: 0,
            sb: [false; 8],
            sr: [false; 7],
            pp_flag_a: false,
            borrow: false,
            read_lo: false,
            read_hi: false,
            memcmp_nz: false,
            memcmp_neg: false,
            load_count: false,
            main_step: 0,
            dst64: 0,
            src64: 0,
            dst_offset: 0,
            src_offset: 0,
            fill_byte: 0,
            cnt_small: 0,
            count: 0,
            rom_l_count64: 0,
            pp_cmp: false,
            sel_w2: false,
            sel_w34: false,
            rd_34: false,
            loop_end: false,
            wr_loop: false,
            wv: [0; 2],
            v3: [0; 2],
            v4: [0; 2],
            rv: [0; 2],
            pw: [F::ZERO; 2],
            op_col: 0,
            b0_col: 0,
            c0_col: 0,
            ext_col: 0,
            xa_col: 0,
            b0_loop: 0,
            ext_loop: 0,
            x_count: 0,
            count_diff: 0,
            loop_count_bytes: 0,
        }
    }
}

impl<F: PrimeField64> DmaCompactRow<F> {
    #[inline(always)]
    fn set_ops(&mut self, op: u8) {
        self.sel_memcpy = op == ZiskOp::DMA_MEMCPY || op == ZiskOp::DMA_XMEMCPY;
        self.sel_memcmp = op == ZiskOp::DMA_MEMCMP || op == ZiskOp::DMA_XMEMCMP;
        self.sel_inputcpy = op == ZiskOp::DMA_INPUTCPY;
        self.sel_memset = op == ZiskOp::DMA_XMEMSET;
        debug_assert!(
            self.sel_memcpy || self.sel_memcmp || self.sel_inputcpy || self.sel_memset,
            "unsupported DMA op 0x{op:02X}"
        );
    }

    #[inline(always)]
    pub fn load_src(&self) -> bool {
        self.sel_memcpy || self.sel_memcmp
    }

    #[inline(always)]
    fn no_src(&self) -> bool {
        self.sel_inputcpy || self.sel_memset
    }

    /// Window offset: `delta = SUM(j * sr[j])`.
    #[inline(always)]
    pub fn delta(&self) -> usize {
        (1..8).find(|&j| self.sr[j - 1]).unwrap_or(0)
    }

    #[inline(always)]
    fn set_delta(&mut self, delta: usize) {
        debug_assert!(delta < 8);
        self.sr = [false; 7];
        if delta > 0 {
            self.sr[delta - 1] = true;
        }
    }

    /// Words a loop row writes: `1 + sel_w2 + 2 * sel_w34`.
    #[inline(always)]
    pub fn words(&self) -> usize {
        1 + usize::from(self.sel_w2) + 2 * usize::from(self.sel_w34)
    }

    /// Operation code that a controller row proves to Main.
    fn main_op(&self, sel_extended: bool) -> u64 {
        let base = if self.sel_memcpy {
            ZiskOp::DMA_MEMCPY as u64
        } else if self.sel_memcmp {
            ZiskOp::DMA_MEMCMP as u64
        } else if self.sel_inputcpy {
            ZiskOp::DMA_INPUTCPY as u64
        } else {
            // __OP_DMA_MEMSET__, which Main always requests as the extended variant
            0xD3
        };
        base + if sel_extended { 6 } else { 0 }
    }

    /// Operation code of the loop.
    fn loop_op(&self) -> u64 {
        if self.sel_memcpy {
            ZiskOp::DMA_XMEMCPY as u64
        } else if self.sel_memcmp {
            OP_DMA_XMEMEQ
        } else if self.sel_inputcpy {
            ZiskOp::DMA_INPUTCPY as u64
        } else {
            ZiskOp::DMA_XMEMSET as u64
        }
    }

    /// Pre operation code of DMA_BUS (the post one adds `OP_DMA_POST_OFFSET`).
    pub fn pre_op(&self) -> u64 {
        if self.sel_memcpy {
            OP_DMA_PRE_XMEMCPY
        } else if self.sel_memcmp {
            OP_DMA_PRE_XMEMCMP
        } else if self.sel_inputcpy {
            OP_DMA_PRE_INPUTCPY
        } else {
            OP_DMA_PRE_XMEMSET
        }
    }

    /// Source byte of the destination byte position `p`: `rb[delta + p]`, and 0 beyond the window
    /// (those positions are never written, see the AIR).
    #[inline(always)]
    fn source_byte(&self, p: usize) -> u8 {
        let idx = self.delta() + p;
        if idx > 14 {
            0
        } else {
            self.rb[idx]
        }
    }

    /// Fills every column defined by an UNGATED constraint of the AIR.
    fn finish(&mut self) {
        // wv[irc] = SUM 256^i * (sb[p] ? rb[delta + p] : pb[p])
        for irc in 0..2 {
            let mut value = 0u64;
            for i in 0..4 {
                let p = irc * 4 + i;
                let byte = if self.sb[p] { self.source_byte(p) } else { self.pb[p] };
                value |= (byte as u64) << (8 * i);
            }
            self.wv[irc] = value;
        }

        let load_src = u64::from(self.load_src());
        let no_src = u64::from(self.no_src());
        let is_ctrl = u64::from(self.is_ctrl);
        let is_loop = u64::from(self.is_loop);
        let sel_extended = u64::from(self.sb[7]);
        let count_bus = self.count + self.count_diff;

        // bus slot 1: the operation proved to Main, or the loop operation of the sequence
        self.op_col = is_ctrl * self.main_op(self.sb[7]) + is_loop * self.loop_op();
        self.b0_col = (self.src64 * 8 + self.src_offset as u64) * load_src + self.count * no_src;
        self.x_count = is_ctrl * sel_extended * count_bus + is_loop * self.count;
        self.ext_col = self.x_count * load_src + self.fill_byte as u64 * u64::from(self.sel_memset);
        self.xa_col = is_loop * self.delta() as u64;

        // the post result shares its columns with the (signed) byte factor, so it is excluded on a
        // pre/post row, exactly as the AIR does
        let post_result_0 = if self.is_pp { 0 } else { self.pw[0].as_canonical_u64() };
        self.c0_col = self.rv[0]
            + post_result_0
            + is_ctrl
                * (1 - u64::from(self.sel_memcmp))
                * (self.dst64 * 8 + self.dst_offset as u64)
            + is_loop * self.dst64 * 8;

        // bus slot 2: only the controller delegates the loop
        let next_src = (self.src64 + u64::from(self.sb[4])) * 8;
        let next_count = is_ctrl * self.loop_count_bytes;
        self.b0_loop = next_src * load_src + next_count * no_src;
        self.ext_loop = next_count * load_src + self.fill_byte as u64 * u64::from(self.sel_memset);
    }

    /// `a[0]` of bus slot 1: the operation proved to Main, or the loop operation of the sequence.
    pub fn prove_a0(&self) -> u64 {
        self.dst64 * 8 + self.dst_offset as u64
    }

    /// `a[0]` of bus slot 2: the loop the controller delegates.
    pub fn assume_a0(&self) -> u64 {
        (self.dst64 + u64::from(self.sb[0])) * 8
    }

    /// Padding row: every column at zero except `seq_end`, which every row that is not a loop row
    /// must have set.
    pub fn padding() -> Self {
        Self { pp_flag_a: true, ..Default::default() }
    }

    /// Whether this is a pre/post row that writes from the byte position 0, that is, a post row.
    /// The AIR reads it from the mask: `is_post = sb[0]`.
    #[inline(always)]
    pub fn is_post(&self) -> bool {
        self.is_pp && self.sb[0]
    }

    /// Controller row of an operation. `count_bus` is the length that travels to Main, which for a
    /// memcmp is larger than the compared one.
    #[allow(clippy::too_many_arguments)]
    pub fn ctrl(
        op: u8,
        dst: u32,
        src: u32,
        step: u64,
        encoded: u64,
        count_bus: u32,
        pre_result: [u64; 2],
        post_result: [u64; 2],
    ) -> Self {
        let mut row =
            Self { is_ctrl: true, pp_flag_a: true, main_step: step, ..Default::default() };
        row.set_ops(op);

        let count = DmaInfo::get_count(encoded) as u64;
        let dst_offset = (dst & 0x07) as u8;
        let src_offset = if row.load_src() { (src & 0x07) as u8 } else { 0 };
        let pre_count = DmaInfo::get_pre_count(encoded);
        let post_count = DmaInfo::get_post_count(encoded);
        let loop_words = DmaInfo::get_loop_count(encoded) as u64;
        let count_lt_256 = count < 256;
        let h_count = (count >> 8).wrapping_sub(u64::from(!count_lt_256));

        row.count = count;
        row.count_diff = count_bus as u64 - count;
        row.loop_count_bytes = loop_words * 8;
        row.dst64 = (dst >> 3) as u64;
        row.src64 = (src >> 3) as u64;
        row.dst_offset = dst_offset;
        row.src_offset = src_offset;
        row.fill_byte = if row.sel_memset { DmaInfo::get_fill_byte(encoded) } else { 0 };
        row.cnt_small = pre_count as u8;
        row.rom_l_count64 = loop_words.wrapping_sub(h_count * 32);
        // the window offset of the loop; DmaRom gives 0 when there is no source
        row.set_delta(if row.load_src() {
            DmaInfo::get_loop_src_offset(encoded) as usize
        } else {
            0
        });

        // the controller byte decomposes count, count_diff and both addresses in the byte pool
        let count_diff = row.count_diff;
        for i in 0..4 {
            row.rb[i] = (count >> (8 * i)) as u8;
            row.rb[4 + i] = (count_diff >> (8 * i)) as u8;
        }
        row.pb[0] = ((dst >> 3) & 0x1F) as u8;
        row.pb[4] = ((src >> 3) & 0x1F) as u8;
        for i in 1..4 {
            row.pb[i] = (dst >> (8 * i)) as u8;
            row.pb[4 + i] = (src >> (8 * i)) as u8;
        }

        let sel_extended =
            matches!(op, ZiskOp::DMA_XMEMCPY | ZiskOp::DMA_XMEMCMP | ZiskOp::DMA_XMEMSET);
        // the result flags only exist for a memcmp (the fill byte of a memset lives in the same
        // bits of the encoded value)
        row.sb = [
            pre_count > 0,
            loop_words > 0,
            post_count > 0,
            count_lt_256,
            DmaInfo::get_src64_inc_by_pre(encoded) > 0,
            row.sel_memcmp && DmaInfo::get_memcmp_pre_result_nz(encoded),
            row.sel_memcmp && DmaInfo::get_memcmp_post_result_nz(encoded),
            sel_extended,
        ];

        row.load_count = (row.load_src() || row.sel_memset) && !sel_extended;
        row.rv = pre_result;
        row.pw = [F::from_u64(post_result[0]), F::from_u64(post_result[1])];

        row.finish();
        row
    }

    /// Pre or post row: writes `count` bytes of ONE destination word.
    ///
    /// `dst` and `src` are the addresses the controller sent on DMA_BUS (the source one is the word
    /// that holds the first byte to read, `src_values[0]`, and `src_values[1]` the next word when
    /// it is read; `src` is 0 for a memset or an inputcpy, whose `src_values[0]` holds the fill
    /// word or the free input bytes in place). `memcmp` carries `(nz, negative, abs_diff)` when
    /// this row resolves the first different byte of a memcmp.
    #[allow(clippy::too_many_arguments)]
    pub fn pre_post(
        op: u8,
        is_post: bool,
        dst: u64,
        src: u64,
        step: u64,
        count: usize,
        src_values: [u64; 2],
        dst_pre_value: u64,
        fill_byte: u8,
        memcmp: Option<(bool, bool, u8)>,
    ) -> Self {
        debug_assert!((1..=8).contains(&count));

        // every row that is not a loop row has seq_end set
        let mut row = Self { is_pp: true, pp_flag_a: true, main_step: step, ..Default::default() };
        row.set_ops(op);

        let dst_offset = (dst & 0x07) as usize;
        let src_offset = if row.load_src() { (src & 0x07) as usize } else { 0 };
        debug_assert!(is_post == (dst_offset == 0), "a post row is the one with dst_offset 0");
        debug_assert!(dst_offset + count <= 8);

        // The window base moves one word backwards when the first byte to write comes from the
        // previous word, which is what makes one single byte selection rule serve every row kind.
        // Without a source (memset, inputcpy) the bytes are taken in place: no offset, no borrow.
        let borrow = row.load_src() && dst_offset > src_offset;
        let delta = if row.load_src() { (src_offset + 8 - dst_offset) & 0x07 } else { 0 };
        let last_position = dst_offset + count - 1;
        let read_lo = row.load_src() && !borrow;
        let read_hi = row.load_src() && (delta + last_position) > 7;

        row.dst64 = dst >> 3;
        row.src64 = (src >> 3) - u64::from(borrow);
        row.dst_offset = dst_offset as u8;
        row.src_offset = src_offset as u8;
        row.cnt_small = count as u8;
        row.borrow = borrow;
        row.read_lo = read_lo;
        row.read_hi = read_hi;
        row.fill_byte = if row.sel_memset { fill_byte } else { 0 };
        row.set_delta(delta);

        // mask of the written positions
        for (p, sb) in row.sb.iter_mut().enumerate() {
            *sb = p >= dst_offset && p <= last_position;
        }

        // the window: a memset fills it, an inputcpy takes the free input in place, and a memcpy or
        // memcmp reads one or two words. With borrow the low half is not read at all, and the word
        // the controller pointed at lands in the high half.
        if row.sel_memset {
            row.rb = [fill_byte; 16];
        } else if borrow {
            row.rb[8..16].copy_from_slice(&src_values[0].to_le_bytes());
        } else {
            row.rb[0..8].copy_from_slice(&src_values[0].to_le_bytes());
            if read_hi {
                row.rb[8..16].copy_from_slice(&src_values[1].to_le_bytes());
            }
        }
        row.pb.copy_from_slice(&dst_pre_value.to_le_bytes());

        if let Some((nz, negative, abs_diff)) = memcmp {
            debug_assert!(row.sel_memcmp);
            debug_assert!(!nz || abs_diff > 0, "a difference cannot be zero");
            debug_assert!(nz || (!negative && abs_diff == 0), "no difference, no sign");
            row.memcmp_nz = nz;
            row.memcmp_neg = negative;
            row.abs_diff = abs_diff;
            row.pp_cmp = true;

            if nz {
                // the factor places the difference at the byte that differs (the last compared
                // one); without a difference DmaCompactTable gives a zero factor
                let (l_factor, h_factor) = if last_position < 4 {
                    (1u64 << (8 * last_position), 0)
                } else {
                    (0, 1u64 << (8 * (last_position - 4)))
                };
                let factor = |v: u64| {
                    let f = F::from_u64(v);
                    if negative && v != 0 {
                        -f
                    } else {
                        f
                    }
                };
                row.pw = [factor(l_factor), factor(h_factor)];

                // the memcmp result of the row, as a signed 64-bit value
                row.rv = [
                    if negative { 0x1_0000_0000 - abs_diff as u64 } else { abs_diff as u64 },
                    if negative { 0xFFFF_FFFF } else { 0 },
                ];
            }
        } else {
            debug_assert!(!row.sel_memcmp, "a memcmp row needs its memcmp info");
        }

        row.finish();
        row
    }

    /// The loop rows of one operation, in sequence order.
    ///
    /// `values` are the source words the loop reads: `loop_words` of them for an aligned operation,
    /// and `loop_words + 1` for an unaligned one, because its last row is an extra read only row
    /// (the same shape `DmaUnaligned` has). A memset needs none.
    #[allow(clippy::too_many_arguments)]
    pub fn loop_sequence(
        op: u8,
        dst64: u64,
        src64: u64,
        step: u64,
        loop_words: usize,
        delta: usize,
        values: &[u64],
        fill_byte: u8,
    ) -> Vec<Self> {
        if loop_words == 0 {
            return Vec::new();
        }
        let unaligned = delta != 0;
        debug_assert!(delta < 8);

        let word = |i: usize| -> u64 {
            if op == ZiskOp::DMA_XMEMSET {
                u64::from_le_bytes([fill_byte; 8])
            } else {
                values.get(i).copied().unwrap_or(0)
            }
        };

        let mut rows = Vec::new();
        let mut done = 0usize;
        while done < loop_words {
            // an unaligned row always does one word: its high half is the next row's read
            let words =
                if unaligned { 1 } else { DmaCompactLayout::loop_row_words(op, loop_words - done) };

            let mut row = Self { is_loop: true, main_step: step, ..Default::default() };
            row.set_ops(op);
            row.dst64 = dst64 + done as u64;
            row.src64 = src64 + done as u64;
            row.count = ((loop_words - done) * 8) as u64;
            row.fill_byte = if row.sel_memset { fill_byte } else { 0 };
            row.sb = [true; 8];
            row.set_delta(delta);
            row.sel_w2 = words >= 2;
            row.sel_w34 = words == 4;
            row.read_lo = row.load_src();
            row.read_hi = row.load_src() && row.sel_w2;
            row.rd_34 = row.load_src() && row.sel_w34;
            row.wr_loop = true;

            row.rb[0..8].copy_from_slice(&word(done).to_le_bytes());
            if unaligned || words >= 2 {
                // unaligned: the bytes of the next row, which is what @[window_from_next] checks;
                // aligned: word 2 of this row, read from memory
                row.rb[8..16].copy_from_slice(&word(done + 1).to_le_bytes());
            }
            if words == 4 {
                let w3 = word(done + 2);
                let w4 = word(done + 3);
                row.v3 = [w3 as u32 as u64, w3 >> 32];
                row.v4 = [w4 as u32 as u64, w4 >> 32];
            }
            if row.sel_memset {
                // a memset row holds the fill byte in every source byte and the fill word in the
                // words 3 and 4, whether the row uses them or not (the AIR forces it)
                let fill_dword = u32::from_le_bytes([fill_byte; 4]) as u64;
                row.rb = [fill_byte; 16];
                row.v3 = [fill_dword, fill_dword];
                row.v4 = [fill_dword, fill_dword];
            }

            done += words;
            // an aligned sequence closes on its last writing row
            if !unaligned && done == loop_words {
                row.pp_flag_a = true;
                row.loop_end = true;
            }
            row.finish();
            rows.push(row);
        }

        if unaligned {
            // the extra row that only reads the source word the previous row needed
            let mut row = Self { is_loop: true, main_step: step, ..Default::default() };
            row.set_ops(op);
            row.dst64 = dst64 + loop_words as u64;
            row.src64 = src64 + loop_words as u64;
            row.count = 0;
            row.fill_byte = if row.sel_memset { fill_byte } else { 0 };
            row.sb = [true; 8];
            row.set_delta(delta);
            row.read_lo = row.load_src();
            row.pp_flag_a = true;
            row.loop_end = true;
            row.rb[0..8].copy_from_slice(&word(loop_words).to_le_bytes());
            row.finish();
            rows.push(row);
        }

        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fields::Goldilocks;

    type Row = DmaCompactRow<Goldilocks>;

    const BASE: u64 = 0xA000_1000;

    /// Word `i` of a synthetic memory, so every byte is identifiable by its address.
    fn mem_word(addr: u64) -> u64 {
        u64::from_le_bytes(std::array::from_fn(|i| (addr + i as u64) as u8))
    }

    /// An aligned sequence: transitions, latches, closure and the words it writes.
    #[test]
    fn aligned_sequence() {
        let words = 11; // 4 + 4 + 2 + 1
        let dst64 = (BASE + 0x800) >> 3;
        let src64 = BASE >> 3;
        let values: Vec<u64> = (0..words).map(|i| mem_word(src64 * 8 + i as u64 * 8)).collect();

        let rows = Row::loop_sequence(ZiskOp::DMA_XMEMCPY, dst64, src64, 7, words, 0, &values, 0);

        assert_eq!(rows.len(), 4);
        assert_eq!(rows.iter().map(|r| r.words()).sum::<usize>(), words);

        for (i, row) in rows.iter().enumerate() {
            // no unaligned row, so every row writes
            assert!(row.wr_loop);
            assert_eq!(row.delta(), 0);
            // seq_end only on the last one, and there the count is exactly what the row does
            assert_eq!(row.pp_flag_a, i == rows.len() - 1);
            if row.pp_flag_a {
                assert_eq!(row.count, 8 * row.words() as u64, "closure of the sequence");
            }
            // transitions against the previous row
            if i > 0 {
                let prev = &rows[i - 1];
                assert_eq!(row.count, prev.count - 8 * prev.words() as u64);
                assert_eq!(row.dst64, prev.dst64 + prev.words() as u64);
                assert_eq!(row.src64, prev.src64 + prev.words() as u64);
                // latches
                assert_eq!(row.main_step, prev.main_step);
                assert_eq!(row.delta(), prev.delta());
                assert_eq!(row.sel_memcpy, prev.sel_memcpy);
                assert_eq!(row.fill_byte, prev.fill_byte);
            }
        }

        // an aligned row writes what it read, word by word
        let first = &rows[0];
        assert_eq!(first.wv, [values[0] as u32 as u64, values[0] >> 32]);
        assert_eq!(first.v3, [values[2] as u32 as u64, values[2] >> 32]);
        assert_eq!(first.v4, [values[3] as u32 as u64, values[3] >> 32]);
        assert!(first.read_lo && first.read_hi && first.rd_34, "4 words need 4 reads");
    }

    /// An unaligned sequence: one word per row, the window of a row is the read of the next one,
    /// the extra read only row closes it, and the bytes written are the right ones.
    #[test]
    fn unaligned_sequence() {
        let words = 3;
        let delta = 5;
        let dst64 = (BASE + 0x800) >> 3;
        let src64 = BASE >> 3;
        // an unaligned sequence reads one word more than it writes
        let values: Vec<u64> = (0..=words).map(|i| mem_word(src64 * 8 + i as u64 * 8)).collect();

        let rows =
            Row::loop_sequence(ZiskOp::DMA_XMEMCPY, dst64, src64, 7, words, delta, &values, 0);

        assert_eq!(rows.len(), words + 1, "one extra read only row");
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.words(), 1, "an unaligned row does one word");
            assert!(row.read_lo && !row.read_hi, "one read per row, never a second one");
            let is_tail = i == rows.len() - 1;
            assert_eq!(row.wr_loop, !is_tail, "the tail row does not write");
            assert_eq!(row.pp_flag_a, is_tail);
            if is_tail {
                assert_eq!(row.count, 0);
            }
            // @[window_from_next]: the high half is the read of the next row
            if let Some(next) = rows.get(i + 1) {
                assert_eq!(row.rb[8..16], next.rb[0..8], "window from the next row at {i}");
            }
        }

        // the bytes written by row i are the source bytes at src64 * 8 + delta + 8i + p
        for (i, row) in rows.iter().take(words).enumerate() {
            let written = u128::from(row.wv[0]) | (u128::from(row.wv[1]) << 32);
            for p in 0..8 {
                let addr = src64 * 8 + delta as u64 + 8 * i as u64 + p as u64;
                let byte = (written >> (8 * p)) as u8;
                assert_eq!(byte, addr as u8, "row {i} byte {p}");
            }
        }
    }

    /// The pre/post row picks the right source bytes for every offset combination, in both the
    /// borrow and the non borrow cases, and keeps the bytes it must not write.
    #[test]
    fn pre_post_window() {
        for dst_offset in 0u64..8 {
            for src_offset in 0u64..8 {
                let max_count = if dst_offset == 0 { 8 } else { 8 - dst_offset };
                for count in 1..=max_count {
                    let dst = BASE + 0x800 + dst_offset;
                    let src = BASE + src_offset;
                    let src_values = [mem_word(src & !7), mem_word((src & !7) + 8)];
                    let dst_pre = 0xEEEE_EEEE_EEEE_EEEEu64;

                    let row = Row::pre_post(
                        ZiskOp::DMA_XMEMCPY,
                        dst_offset == 0,
                        dst,
                        src,
                        7,
                        count as usize,
                        src_values,
                        dst_pre,
                        0,
                        None,
                    );

                    // the address of the word the controller pointed at is preserved
                    assert_eq!(
                        (row.src64 + u64::from(row.borrow)) * 8,
                        src & !7,
                        "payload src (do={dst_offset} so={src_offset} c={count})"
                    );
                    // reads: never the word before the source one
                    assert!(!(row.read_lo && row.borrow));

                    let written = u128::from(row.wv[0]) | (u128::from(row.wv[1]) << 32);
                    for p in 0..8u64 {
                        let byte = (written >> (8 * p)) as u8;
                        if p >= dst_offset && p < dst_offset + count {
                            // written byte: comes from src + (p - dst_offset)
                            let addr = src + p - dst_offset;
                            assert_eq!(
                                byte, addr as u8,
                                "do={dst_offset} so={src_offset} c={count} p={p}"
                            );
                        } else {
                            // untouched byte: the previous value of the destination word
                            assert_eq!(byte, (dst_pre >> (8 * p)) as u8);
                        }
                    }
                }
            }
        }
    }

    /// A memcmp pre/post row: the factored difference is exactly what the AIR checks,
    /// `p_value - wv = diff_factor * abs_diff`.
    #[test]
    fn pre_post_memcmp_difference() {
        let dst_offset = 3u64;
        let src_offset = 3u64;
        let count = 4usize;
        let dst = BASE + 0x800 + dst_offset;
        let src = BASE + src_offset;

        // the destination equals the source except in the last compared byte, which is bigger by 7
        let src_word = mem_word(src & !7);
        let last_position = dst_offset as usize + count - 1;
        let dst_word = src_word + ((7u64) << (8 * last_position));

        let row = Row::pre_post(
            ZiskOp::DMA_XMEMCMP,
            false,
            dst,
            src,
            7,
            count,
            [src_word, mem_word((src & !7) + 8)],
            dst_word,
            0,
            Some((true, false, 7)),
        );

        // p_value[i] - wv[i] == diff_factor[i] * abs_diff, in the field
        let p_value = [dst_word as u32 as u64, dst_word >> 32];
        for i in 0..2 {
            let lhs = Goldilocks::from_u64(p_value[i]) - Goldilocks::from_u64(row.wv[i]);
            let rhs = row.pw[i] * Goldilocks::from_u64(row.abs_diff as u64);
            assert_eq!(lhs, rhs, "chunk {i}");
        }
        assert_eq!(row.rv, [7, 0], "positive result");
        assert!(row.pp_cmp && row.memcmp_nz && !row.memcmp_neg);
    }

    /// A memset loop row writes the fill word and reads nothing.
    #[test]
    fn memset_sequence() {
        let rows =
            Row::loop_sequence(ZiskOp::DMA_XMEMSET, (BASE + 0x800) >> 3, 0, 7, 4, 0, &[], 0xAB);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!(!row.read_lo && !row.read_hi && !row.rd_34, "memset reads nothing");
        assert_eq!(row.wv, [0xABAB_ABAB, 0xABAB_ABAB]);
        assert_eq!(row.v3, [0xABAB_ABAB, 0xABAB_ABAB]);
        assert_eq!(row.v4, [0xABAB_ABAB, 0xABAB_ABAB]);
        assert_eq!(row.ext_col, 0xAB, "the fill byte travels in extended_arg");
        assert_eq!(row.rb, [0xAB; 16], "every source byte of the window is the fill byte");
    }

    /// The controller delegates the loop that the first row of the sequence proves.
    #[test]
    fn controller_delegates_the_loop() {
        let count = 8 * 5 + 3;
        let dst = BASE + 0x800;
        let src = BASE;
        let encoded = DmaInfo::encode_memcpy(dst, src, count);
        let ctrl = Row::ctrl(
            ZiskOp::DMA_XMEMCPY,
            dst as u32,
            src as u32,
            7,
            encoded,
            count as u32,
            [0, 0],
            [0, 0],
        );

        assert_eq!(ctrl.prove_a0(), dst);
        assert_eq!(ctrl.b0_col, src);
        assert_eq!(ctrl.ext_col, count as u64);
        assert_eq!(ctrl.c0_col, dst, "a memcpy returns the destination");
        assert_eq!(ctrl.xa_col, 0);
        assert!(ctrl.pp_flag_a, "a row that is not a loop row has seq_end set");

        let rows = Row::loop_sequence(ZiskOp::DMA_XMEMCPY, dst >> 3, src >> 3, 7, 5, 0, &[0; 5], 0);
        assert_eq!(ctrl.assume_a0(), rows[0].prove_a0());
        assert_eq!(ctrl.b0_loop, rows[0].b0_col);
        assert_eq!(ctrl.ext_loop, rows[0].ext_col);
    }
}
