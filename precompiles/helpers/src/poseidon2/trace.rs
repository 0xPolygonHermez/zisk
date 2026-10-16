//! Row values of one permutation block of the `Poseidon2NonNative` AIR
//! (precompiles/poseidon/pil/poseidon2_non_native.pil). Every value is below 2^32, as the AIR
//! stores each one in two 16-bit limbs.

use super::{pow_mod, Poseidon2Params};

/// Most S-box steps (degree 7: x², x³, x⁶, x⁷).
pub const MAX_SBOX_STEPS: usize = 4;

/// One step of the S-box: it squares the previous value or multiplies it by x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SboxStep {
    Square,
    TimesX,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Poseidon2Row<const W: usize> {
    pub chunks: [u32; W],
    pub sbox: [[u32; MAX_SBOX_STEPS]; W],
    pub sbox_q: [[u32; MAX_SBOX_STEPS]; W],
    pub linear_q: [u32; W],
}

impl<const W: usize> Default for Poseidon2Row<W> {
    fn default() -> Self {
        Self {
            chunks: [0; W],
            sbox: [[0; MAX_SBOX_STEPS]; W],
            sbox_q: [[0; MAX_SBOX_STEPS]; W],
            linear_q: [0; W],
        }
    }
}

impl<const W: usize> Poseidon2Params<W> {
    /// Rows per permutation: initial layer, RF + RP rounds and the output row.
    pub const fn clocks(&self) -> usize {
        self.rc_begin.len() + self.rc_partial.len() + self.rc_end.len() + 2
    }

    /// Permutations that fit in a trace of `num_rows` rows (NUM_POSEIDON in the PIL).
    pub const fn num_available(&self, num_rows: usize) -> usize {
        let clocks = self.clocks();
        num_rows / clocks - (num_rows % clocks != 0) as usize
    }

    /// Steps computing x^sbox_degree, matching STEP in the PIL.
    pub fn sbox_steps(&self) -> &'static [SboxStep] {
        use SboxStep::*;
        match self.sbox_degree {
            3 => &[Square, TimesX],                 // x², x³
            5 => &[Square, Square, TimesX],         // x², x⁴, x⁵
            7 => &[Square, TimesX, Square, TimesX], // x², x³, x⁶, x⁷
            d => panic!("unsupported S-box degree {d}"),
        }
    }

    /// Round constant of lane `i` for round `r`, rounds numbered from the first full round.
    fn round_constant(&self, r: usize, i: usize) -> u32 {
        let half = self.rc_begin.len();
        let rp = self.rc_partial.len();
        if r < half {
            self.rc_begin[r][i]
        } else if r < half + rp {
            if i == 0 {
                self.rc_partial[r - half]
            } else {
                0
            }
        } else if r < half + rp + self.rc_end.len() {
            self.rc_end[r - half - rp][i]
        } else {
            0
        }
    }

    /// Values of the `clocks()` rows proving the permutation of `input`, the lanes as they are in
    /// memory (`io_scale * x`, canonical). Every intermediate fits in 63 bits: values are below
    /// 2^31 and quotients below 2^32.
    ///
    /// On the input and output rows, which have no S-box, lane i keeps the memory value in
    /// `sbox[i][1]`, the quotient of its conversion in `sbox_q[i][0]` and its canonical gap in
    /// `sbox[i][0]`.
    pub fn trace_rows(&self, input: &[u32; W]) -> Vec<Poseidon2Row<W>> {
        let p = u64::from(self.p);
        let io_scale = u64::from(self.io_scale);
        let io_inv = u64::from(self.io_scale_inv());
        let half = self.rc_begin.len();
        let first_partial = half + 1;
        let first_end = half + self.rc_partial.len() + 1;
        let last = self.clocks() - 1;
        let sbox_steps = self.sbox_steps();

        // Internal layer as S·a' + K = Σy + d·y + P·q (see the PIL)
        let s = u64::from(pow_mod(self.internal_scale, self.p - 2, self.p));
        let d: [i64; W] = std::array::from_fn(|i| {
            let v = i64::from(self.internal_diag[i]);
            if v > p as i64 / 2 {
                v - p as i64
            } else {
                v
            }
        });
        let bias = (W as u64 + d.iter().copied().max().unwrap().max(0) as u64) * p;

        let mut rows = vec![Poseidon2Row::<W>::default(); last + 1];

        // Memory value m of lane i on the input or output row, with its gap and conversion quotient
        let set_io = |row: &mut Poseidon2Row<W>, i: usize, m: u64, q: u64| {
            row.sbox[i] = [(p - 1 - m) as u32, m as u32, 0, 0];
            row.sbox_q[i][0] = q as u32;
        };

        // Input: m·io_inv = x + P·q
        let mut state = [0u64; W];
        for (i, &m) in input.iter().enumerate() {
            let m = u64::from(m);
            state[i] = m * io_inv % p;
            set_io(&mut rows[0], i, m, m * io_inv / p);
        }

        for (k, row) in rows.iter_mut().enumerate() {
            row.chunks = state.map(|x| x as u32);
            if k == last {
                // Output: y·io_scale = m + P·q
                for (i, &y) in state.iter().enumerate() {
                    set_io(row, i, y * io_scale % p, y * io_scale / p);
                }
                break;
            }

            let full = (1..first_partial).contains(&k) || (first_end..last).contains(&k);
            let partial = (first_partial..first_end).contains(&k);

            // S-box
            let mut y = state;
            for i in 0..W {
                if !(full || (partial && i == 0)) {
                    continue;
                }
                let x = state[i];
                let mut v = x;
                for (k, step) in sbox_steps.iter().enumerate() {
                    let prod = v * if *step == SboxStep::Square { v } else { x };
                    v = prod % p;
                    row.sbox[i][k] = v as u32;
                    row.sbox_q[i][k] = (prod / p) as u32;
                }
                y[i] = v;
            }

            // Linear layer, plus the next round's constant
            if partial {
                let sum: u64 = y.iter().sum();
                for i in 0..W {
                    let rc = u64::from(self.round_constant(k, i));
                    let mixed = (sum as i64 + d[i] * y[i] as i64).rem_euclid(p as i64) as u64;
                    let next = (mixed * u64::from(self.internal_scale) + rc) % p;
                    let k_const = bias + (p - s * rc % p) % p;
                    let lhs = (s * next + k_const) as i64 - sum as i64 - d[i] * y[i] as i64;
                    debug_assert!(lhs >= 0 && lhs % p as i64 == 0);
                    state[i] = next;
                    row.linear_q[i] = (lhs as u64 / p) as u32;
                }
            } else {
                // circ(2·M4, M4, ...): M4 on each block of 4, plus the column sums of all blocks
                let mut t = [0u64; W];
                let mut sums = [0u64; 4];
                for (b, block) in y.chunks_exact(4).enumerate() {
                    for r in 0..4 {
                        t[4 * b + r] = (0..4).map(|c| u64::from(self.m4[r][c]) * block[c]).sum();
                        sums[r] += t[4 * b + r];
                    }
                }
                for i in 0..W {
                    let lin = t[i] + sums[i % 4] + u64::from(self.round_constant(k, i));
                    state[i] = lin % p;
                    row.linear_q[i] = (lin / p) as u32;
                }
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_vectors::KOALABEAR_SP1_VECTORS;
    use super::super::POSEIDON2_KOALABEAR_SP1;

    #[test]
    fn trace_rows_end_in_the_permutation_output() {
        let params = &POSEIDON2_KOALABEAR_SP1;
        assert_eq!(params.clocks(), 30);
        let mont = |v: [u32; 16]| v.map(|x| ((u64::from(x) << 32) % u64::from(params.p)) as u32);
        for (input, expected) in KOALABEAR_SP1_VECTORS {
            let rows = params.trace_rows(&mont(input));
            let last = rows.last().unwrap();
            assert_eq!(rows[0].chunks, input);
            assert_eq!(last.chunks, expected);
            assert_eq!(rows[0].sbox.map(|s| s[1]), mont(input));
            assert_eq!(last.sbox.map(|s| s[1]), mont(expected));
        }
    }
}
