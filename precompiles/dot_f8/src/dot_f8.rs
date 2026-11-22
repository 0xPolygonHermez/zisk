use std::marker::PhantomData;

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::{OPERATION_BUS_DOT_F8_DATA_SIZE, OPERATION_PRECOMPILED_BUS_DATA_SIZE, STEP};
use zisk_core::dot_f8_data_ext_len;
use zisk_definitions::{dot_f8_bits, dot_f8_value, DOT_F8_E2M1, DOT_F8_REPEAT_Y};
use zisk_pil::{DotF8TraceRowOps, ZISK_AIRGROUP_ID};
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

/// Rows of a call of `n` values: 8 multiply-adds per row, one row for an empty call.
pub fn dot_f8_rows(n: u64) -> usize {
    (n as usize / 8).max(1)
}

/// One call from the operation bus. `words` holds x, then y (one block of y with
/// `DOT_F8_REPEAT_Y`).
#[derive(Clone, Debug)]
pub struct DotF8Input {
    pub step: u64,
    pub params: u64,
    /// x, y, n, formats, block, out.
    pub p: [u64; 6],
    pub c: u64,
    pub words: Vec<u64>,
}

impl DotF8Input {
    pub fn new(data: &[u64], words: &[u64]) -> Self {
        assert_eq!(data.len(), OPERATION_BUS_DOT_F8_DATA_SIZE, "dot_f8: wrong header size");
        let h = &data[OPERATION_PRECOMPILED_BUS_DATA_SIZE..];
        assert_eq!(dot_f8_data_ext_len(h), words.len(), "dot_f8: wrong number of vector words");
        Self {
            step: data[STEP],
            params: data[3],
            p: h[..6].try_into().unwrap(),
            c: h[6],
            words: words.to_vec(),
        }
    }
    fn n(&self) -> u64 {
        self.p[2]
    }
    fn formats(&self) -> (u8, u8) {
        (self.p[3] as u8, (self.p[3] >> 8) as u8)
    }
    fn repeat_y(&self) -> bool {
        self.p[3] & DOT_F8_REPEAT_Y != 0
    }
    fn x_words(&self) -> usize {
        (self.n() * dot_f8_bits(self.formats().0) as u64 / 64) as usize
    }
    /// Rows per block; a block word of 0 means one block.
    fn block_rows(&self) -> u64 {
        if self.p[4] == 0 {
            self.n() / 8
        } else {
            self.p[4] / 8
        }
    }
    fn blocks(&self) -> u64 {
        if self.n() == 0 {
            0
        } else {
            self.n() / 8 / self.block_rows()
        }
    }

    /// The block sums, as the AIR and the emulator compute them.
    fn block_sums(&self) -> Vec<i64> {
        let (fx, fy) = self.formats();
        let bytes = |w: &[u64]| w.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
        let (x, y) = (bytes(&self.words[..self.x_words()]), bytes(&self.words[self.x_words()..]));
        let block = 8 * self.block_rows() as usize;
        let y_start = |b: usize| if self.repeat_y() { 0 } else { b * block };
        (0..self.blocks() as usize)
            .map(|b| {
                (0..block)
                    .map(|i| {
                        dot_f8_value(fx, &x, b * block + i) as i64
                            * dot_f8_value(fy, &y, y_start(b) + i) as i64
                    })
                    .sum()
            })
            .collect()
    }
}

pub struct DotF8SM<F: PrimeField64> {
    _phantom: PhantomData<F>,
}

/// `v` as a canonical field element.
fn field<F: PrimeField64>(v: i128) -> u64 {
    let f = if v >= 0 { F::from_u64(v as u64) } else { -F::from_u64(v.unsigned_abs() as u64) };
    f.as_canonical_u64()
}

/// The inverse of `v`, or 0 for 0.
fn inv<F: PrimeField64>(v: i128) -> u64 {
    let f = F::from_u64(field::<F>(v));
    if f.is_zero() {
        0
    } else {
        f.inverse().as_canonical_u64()
    }
}

/// i64 limbs `[l0, l1], [h0, h1], sign` of `v`.
fn limbs(v: i64) -> ([u64; 2], [u64; 2], u64) {
    let u = v as u64;
    let (lo, hi) = (u & 0xffff_ffff, u >> 32);
    ([lo & 0xffff, lo >> 16], [hi & 0xffff, (hi >> 16) & 0x7fff], hi >> 31)
}

impl<F: PrimeField64> DotF8SM<F> {
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self { _phantom: PhantomData })
    }

    /// Rows of one call, following `precompiles/dot_f8/pil/dot_f8.pil`.
    fn fill_call<R: DotF8TraceRowOps<F>>(input: &DotF8Input, rows: &mut [R]) {
        let (fx, fy) = input.formats();
        let (ex, ey) = (u64::from(fx == DOT_F8_E2M1), u64::from(fy == DOT_F8_E2M1));
        let r = (input.n() / 8) as i128;
        let nz = u64::from(r != 0);
        let bk = input.block_rows() as i128;
        let ow = (input.p[5] / 8) as i128;
        let (cl, ch, csg) = limbs(input.c as i64);
        let sums = input.block_sums();
        let (xw0, yw0) = (input.p[0] / 8, input.p[1] / 8);
        let rep = input.repeat_y();
        let (xs, ys) = input.words.split_at(input.x_words());
        let (mut bidx, mut accb, mut tot, mut ob, mut block) = (0i128, 0i128, 0i128, ow, 0usize);
        for (i, row) in rows.iter_mut().enumerate() {
            let par = (i % 2) as u64;
            let (lx, ly) = (1 - ex * par, 1 - ey * par);
            // With rep, y restarts with each block (bidx counts the rows of the block).
            let yi = if rep { bidx as usize } else { i };
            let yr = rep && i > 0 && bidx == 0;
            let (xwi, ywi) = (if ex == 1 { i / 2 } else { i }, if ey == 1 { yi / 2 } else { yi });
            let word = |ws: &[u64], k: usize| if nz == 1 { ws[k] } else { 0 };
            let (xword, yword) = (word(xs, xwi), word(ys, ywi));
            let codes = |w: u64, e: u64| -> [u64; 8] {
                std::array::from_fn(|j| {
                    if e == 1 {
                        (w >> (32 * par + 4 * j as u64)) & 15
                    } else {
                        (w >> (8 * j)) & 255
                    }
                })
            };
            let (xc, yc) = (codes(xword, ex), codes(yword, ey));
            let value = |f: u8, c: u64| dot_f8_value(f, &[c as u8], 0) as i128;
            let xv: [i128; 8] = std::array::from_fn(|j| value(fx, xc[j]));
            let yv: [i128; 8] = std::array::from_fn(|j| value(fy, yc[j]));
            let prod: i128 = (0..8).map(|j| xv[j] * yv[j]).sum();
            let rem = r - 1 - i as i128;
            let brem = bk - 1 - bidx;
            let ce = u64::from(rem == 0);
            let be = nz * u64::from(brem == 0);
            let w = be * u64::from(ow != 0);
            let s = accb + prod;
            let t = tot + prod;

            row.set_in_use(true);
            row.set_start(i == 0);
            row.set_end(nz == 0 || ce == 1);
            row.set_active(nz == 1);
            row.set_nz(nz == 1);
            row.set_ce(nz == 1 && ce == 1);
            row.set_be(be == 1);
            row.set_bz(input.p[4] == 0);
            row.set_oz(ow == 0);
            row.set_par(par == 1);
            row.set_lx(lx == 1);
            row.set_ly(ly == 1);
            row.set_w(w == 1);
            row.set_all_fx(&[fx & 1 == 1, fx & 2 == 2]);
            row.set_all_fy(&[fy & 1 == 1, fy & 2 == 2]);
            row.set_ex(ex == 1);
            row.set_ey(ey == 1);
            row.set_csg(csg == 1);
            row.set_rep(rep);
            row.set_yr(yr);
            row.set_yb(field::<F>(yw0 as i128));
            row.set_step(field::<F>(input.step as i128));
            row.set_pw(field::<F>(input.params as i128 / 8));
            row.set_R(field::<F>(r));
            row.set_r_inv(inv::<F>(r));
            row.set_idx(field::<F>(i as i128));
            row.set_rem_inv(inv::<F>(rem));
            row.set_Bk(field::<F>(bk));
            row.set_bidx(field::<F>(bidx));
            row.set_brem_inv(inv::<F>(brem));
            row.set_ow(field::<F>(ow));
            row.set_ow_inv(inv::<F>(ow));
            row.set_ob(field::<F>(ob));
            row.set_xw(field::<F>((xw0 + xwi as u64) as i128));
            row.set_yw(field::<F>((yw0 + ywi as u64) as i128));
            row.set_all_x_word(&[xword & 0xffff_ffff, xword >> 32]);
            row.set_all_y_word(&[yword & 0xffff_ffff, yword >> 32]);
            row.set_all_xc(&xc);
            row.set_all_xv(&xv.map(field::<F>));
            row.set_all_yc(&yc);
            row.set_all_yv(&yv.map(field::<F>));
            row.set_accb(field::<F>(accb));
            row.set_s(field::<F>(s));
            row.set_tot(field::<F>(tot));
            row.set_t(field::<F>(t));
            row.set_all_cl(&cl);
            row.set_all_ch(&ch);
            if w == 1 {
                assert_eq!(s, sums[block] as i128, "dot_f8: block sum");
                let (bl, bh, bsg) = limbs(sums[block]);
                row.set_all_bl(&bl);
                row.set_all_bh(&bh);
                row.set_bsg(bsg == 1);
            }
            if be == 1 {
                block += 1;
            }
            ob += be as i128;
            (bidx, accb) = if be == 1 { (0, 0) } else { (bidx + 1, s) };
            tot = t;
        }
        if nz == 1 {
            assert_eq!(tot as i64 as u64, input.c, "dot_f8: total differs from the emulator");
        }
    }

    /// An empty row: one n = 0 call that is not in use.
    fn padding<R: DotF8TraceRowOps<F>>() -> R {
        let mut row = R::default();
        row.set_start(true);
        row.set_end(true);
        row.set_oz(true);
        row.set_lx(true);
        row.set_ly(true);
        row.set_rem_inv((-F::ONE).as_canonical_u64());
        row
    }

    pub fn compute_witness<R: DotF8TraceRowOps<F>, const NUM_ROWS: usize, const AIR_ID: usize>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<DotF8Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = GenericTrace::<R, NUM_ROWS, ZISK_AIRGROUP_ID, AIR_ID>::new_from_vec_zeroes(
            trace_buffer,
        )?;
        let rows = &mut trace.buffer;
        let mut offset = 0;
        for input in inputs.iter().flatten() {
            let count = dot_f8_rows(input.n());
            assert!(offset + count <= rows.len(), "dot_f8: inputs exceed the trace");
            Self::fill_call(input, &mut rows[offset..offset + count]);
            offset += count;
        }
        let padding = Self::padding::<R>();
        rows[offset..].fill(padding);
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}

impl<F: PrimeField64> PrecompileMemInputs for DotF8SM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        _only_counters: bool,
        mem: &mut P,
    ) {
        let (header, words) = data.split_at(OPERATION_BUS_DOT_F8_DATA_SIZE);
        let input = DotF8Input::new(header, words);
        for (i, v) in input.p.iter().enumerate() {
            MemBusHelpers::mem_aligned_read(addr_main + 8 * i as u32, step_main, *v, mem);
        }
        let (xs, ys) = input.words.split_at(input.x_words());
        // With DOT_F8_REPEAT_Y the state machine reads y once per block.
        let y_reads = if input.repeat_y() { input.blocks() as usize } else { 1 };
        let reads = std::iter::once((input.p[0], xs)).chain((0..y_reads).map(|_| (input.p[1], ys)));
        for (base, ws) in reads {
            for (k, v) in ws.iter().enumerate() {
                MemBusHelpers::mem_aligned_read(base as u32 + 8 * k as u32, step_main, *v, mem);
            }
        }
        if input.p[5] != 0 {
            for (b, s) in input.block_sums().iter().enumerate() {
                MemBusHelpers::mem_aligned_write(
                    input.p[5] as u32 + 8 * b as u32,
                    step_main,
                    *s as u64,
                    mem,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proofman_fields::Goldilocks;
    use zisk_definitions::{dot_f8_reference_with, DOT_F8_FORMATS};
    use zisk_pil::DotF8TraceRow;

    use super::*;

    type F = Goldilocks;

    #[test]
    fn limbs_rebuild_the_value() {
        for v in [0i64, 1, -1, (1 << 62) - 1, -(1 << 62), 0x1234_5678_9abc, -0x1234_5678_9abc] {
            let ([l0, l1], [h0, h1], sg) = limbs(v);
            assert!(l0 < 1 << 16 && l1 < 1 << 16 && h0 < 1 << 16 && h1 < 1 << 15 && sg < 2);
            let rebuilt = (l0 + (l1 << 16)) as i128
                + ((h0 + (h1 << 16) + (sg << 31)) as i128) * (1 << 32)
                - (sg as i128) * (1 << 64);
            assert_eq!(rebuilt, v as i128);
        }
    }

    /// One call as the bus carries it; `c` comes from the reference.
    fn call(fx: u8, fy: u8, n: u64, block: u64, out: u64, seed: u64) -> DotF8Input {
        call_with(fx, fy, n, block, out, seed, false)
    }

    fn call_with(
        fx: u8,
        fy: u8,
        n: u64,
        block: u64,
        out: u64,
        seed: u64,
        repeat_y: bool,
    ) -> DotF8Input {
        let mut s = seed;
        let mut word = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            s
        };
        let xw = (n * dot_f8_bits(fx) as u64 / 64) as usize;
        let y_values = if repeat_y { block.min(n) } else { n };
        let yw = (y_values * dot_f8_bits(fy) as u64 / 64) as usize;
        let words: Vec<u64> = (0..xw + yw).map(|_| word()).collect();
        let bytes = |w: &[u64]| w.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
        let size = if block == 0 { n } else { block } as usize;
        let c = dot_f8_reference_with(
            fx,
            &bytes(&words[..xw]),
            fy,
            &bytes(&words[xw..]),
            n as usize,
            size.max(1),
            repeat_y,
            None,
        );
        let mut data = vec![0u64; OPERATION_BUS_DOT_F8_DATA_SIZE];
        data[3] = 0x1000;
        data[STEP] = 77;
        data[OPERATION_PRECOMPILED_BUS_DATA_SIZE..].copy_from_slice(&[
            0x2000,
            0x8000,
            n,
            fx as u64 | (fy as u64) << 8 | if repeat_y { DOT_F8_REPEAT_Y } else { 0 },
            block,
            out,
            c as u64,
        ]);
        DotF8Input::new(&data, &words)
    }

    fn fill(input: &DotF8Input) -> Vec<DotF8TraceRow<F>> {
        let mut rows = vec![DotF8TraceRow::<F>::default(); dot_f8_rows(input.n())];
        DotF8SM::<F>::fill_call(input, &mut rows);
        rows
    }

    #[test]
    fn every_format_pair_matches_the_reference() {
        // fill_call checks each block sum and the total against the reference.
        for fx in 0..DOT_F8_FORMATS {
            for fy in 0..DOT_F8_FORMATS {
                for (block, out) in [(0, 0), (0, 0x9000), (16, 0x9000), (32, 0)] {
                    let input = call(fx, fy, 64, block, out, (fx * 4 + fy) as u64 + block);
                    let rows = fill(&input);
                    assert_eq!(rows.len(), 8);
                    assert!(rows[7].get_end() && rows.iter().take(7).all(|r| !r.get_end()));
                    let blocks = rows.iter().filter(|r| r.get_be()).count() as u64;
                    assert_eq!(blocks, 64 / if block == 0 { 64 } else { block });
                    let writes = rows.iter().filter(|r| r.get_w()).count() as u64;
                    assert_eq!(writes, if out == 0 { 0 } else { blocks });
                }
            }
        }
    }

    #[test]
    fn repeat_y_restarts_y_with_each_block() {
        // fill_call checks each block sum and the total against the reference.
        for fx in 0..DOT_F8_FORMATS {
            for fy in 0..DOT_F8_FORMATS {
                for (block, out) in [(16, 0x9000), (32, 0), (64, 0x9000)] {
                    let input = call_with(fx, fy, 64, block, out, fx as u64 * 4 + fy as u64, true);
                    let rows = fill(&input);
                    let y0 = 0x8000 / 8;
                    let per_row = if fy == DOT_F8_E2M1 { 2 } else { 1 };
                    for (i, r) in rows.iter().enumerate() {
                        let k = i % (block as usize / 8);
                        assert!(r.get_rep() && r.get_yb() == y0);
                        assert_eq!(r.get_yr(), i > 0 && k == 0);
                        assert_eq!(r.get_yw(), y0 + (k / per_row) as u64);
                    }
                }
            }
        }
    }

    /// Counts the memory reads `generate` emits, by address.
    #[derive(Default)]
    struct Reads(HashMap<u64, usize>);
    impl MemProcessor for Reads {
        fn process_mem_data(&mut self, data: &[u64; 7]) {
            if data[0] == 1 {
                // MEMORY_LOAD_OP
                *self.0.entry(data[1]).or_default() += 1;
            }
        }
        fn skip_addr(&mut self, _: u32) -> bool {
            false
        }
        fn skip_addr_range(&mut self, _: u32, _: u32) -> bool {
            false
        }
    }

    #[test]
    fn repeat_y_reads_y_once_per_block() {
        // E4M3 x E2M1: 4 blocks of 32 values, y one block of 2 words.
        let input = call_with(1, DOT_F8_E2M1, 128, 32, 0x9000, 3, true);
        assert_eq!(input.words.len(), 16 + 2);
        let mut data = vec![0u64; OPERATION_BUS_DOT_F8_DATA_SIZE];
        data[OPERATION_PRECOMPILED_BUS_DATA_SIZE..]
            .copy_from_slice(&[&input.p[..], &[input.c]].concat());
        data.extend(&input.words);
        let mut reads = Reads::default();
        DotF8SM::<F>::generate(0x1000, 77, &data, false, &mut reads);
        assert_eq!(reads.0[&0x8000], 4);
        assert_eq!(reads.0[&0x8008], 4);
        assert_eq!(reads.0.get(&0x8010), None);
        assert_eq!(reads.0[&0x2000], 1);
        assert_eq!(reads.0[&0x1000], 1);
    }

    #[test]
    fn empty_call_is_one_inactive_row() {
        let rows = fill(&call(1, 3, 0, 0, 0x9000, 5));
        assert_eq!(rows.len(), 1);
        assert!(rows[0].get_end() && !rows[0].get_active() && !rows[0].get_w());
        assert_eq!(rows[0].get_t(), 0);
    }
}
