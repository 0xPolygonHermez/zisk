//! Poseidon2 over 31-bit prime fields (KoalaBear, BabyBear, ...), on canonical `u32` lanes.
//!
//! One `Poseidon2Params` value describes an instance; the permutation is generic over it:
//! - external layer: `circ(2*M4, M4, ..., M4)` (Plonky3 layout), also applied before round 1;
//! - internal layer: `internal_scale * (J + diag(internal_diag))`;
//! - `rc_begin.len()` full rounds, `rc_partial.len()` partial rounds, `rc_end.len()` full rounds.

mod koalabear_sp1;
#[cfg(test)]
mod test_vectors;
mod trace;

pub use koalabear_sp1::POSEIDON2_KOALABEAR_SP1;

pub struct Poseidon2Params<const W: usize> {
    pub p: u32,
    pub sbox_degree: u32,
    pub m4: [[u32; 4]; 4],
    pub internal_scale: u32,
    pub internal_diag: [u32; W],
    pub rc_begin: &'static [[u32; W]],
    pub rc_partial: &'static [u32],
    pub rc_end: &'static [[u32; W]],
}

impl<const W: usize> Poseidon2Params<W> {
    /// State width (number of lanes).
    pub const fn width(&self) -> usize {
        W
    }

    /// Index of the first lane that is not a canonical field element, if any.
    fn first_noncanonical(&self, state: &[u32; W]) -> Option<usize> {
        state.iter().position(|&x| x >= self.p)
    }

    /// Lanes of a state stored two per little-endian `u64` word (`W / 2` words), as guests pass it.
    pub fn lanes(words: &[u64]) -> [u32; W] {
        assert_eq!(words.len() * 2, W);
        std::array::from_fn(|i| (words[i / 2] >> (32 * (i % 2))) as u32)
    }

    /// Permutes a state stored two lanes per word (see `lanes`). On a non-canonical lane returns its
    /// index and leaves `words` untouched.
    pub fn permute_words(&self, words: &mut [u64]) -> Result<(), usize> {
        let mut state = Self::lanes(words);
        if let Some(i) = self.first_noncanonical(&state) {
            return Err(i);
        }
        self.permute(&mut state);
        for (word, pair) in words.iter_mut().zip(state.chunks_exact(2)) {
            *word = u64::from(pair[0]) | (u64::from(pair[1]) << 32);
        }
        Ok(())
    }

    /// Permutes `state` in place. Every lane must be canonical.
    pub fn permute(&self, state: &mut [u32; W]) {
        debug_assert!(self.first_noncanonical(state).is_none());
        self.external_layer(state);
        for rc in self.rc_begin {
            self.full_round(state, rc);
        }
        for &rc in self.rc_partial {
            state[0] = self.sbox(self.add(state[0], rc));
            self.internal_layer(state);
        }
        for rc in self.rc_end {
            self.full_round(state, rc);
        }
    }

    fn full_round(&self, state: &mut [u32; W], rc: &[u32; W]) {
        for (x, &c) in state.iter_mut().zip(rc) {
            *x = self.sbox(self.add(*x, c));
        }
        self.external_layer(state);
    }

    fn external_layer(&self, state: &mut [u32; W]) {
        let p = u64::from(self.p);
        let mut y = [0u64; W];
        for (block, out) in state.chunks_exact(4).zip(y.chunks_exact_mut(4)) {
            for (row, o) in self.m4.iter().zip(out.iter_mut()) {
                *o = row.iter().zip(block).map(|(&m, &x)| u64::from(m) * u64::from(x)).sum::<u64>()
                    % p;
            }
        }
        let mut sums = [0u64; 4];
        for (i, v) in y.iter().enumerate() {
            sums[i % 4] += v;
        }
        for (i, x) in state.iter_mut().enumerate() {
            *x = ((y[i] + sums[i % 4]) % p) as u32;
        }
    }

    fn internal_layer(&self, state: &mut [u32; W]) {
        let p = u64::from(self.p);
        let sum = state.iter().map(|&x| u64::from(x)).sum::<u64>() % p;
        for (x, &d) in state.iter_mut().zip(&self.internal_diag) {
            let v = (sum + u64::from(d) * u64::from(*x)) % p;
            *x = self.mul(v as u32, self.internal_scale);
        }
    }

    fn sbox(&self, x: u32) -> u32 {
        (1..self.sbox_degree).fold(x, |acc, _| self.mul(acc, x))
    }

    fn add(&self, a: u32, b: u32) -> u32 {
        ((u64::from(a) + u64::from(b)) % u64::from(self.p)) as u32
    }

    fn mul(&self, a: u32, b: u32) -> u32 {
        (u64::from(a) * u64::from(b) % u64::from(self.p)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::test_vectors::KOALABEAR_SP1_VECTORS;
    use super::*;

    #[test]
    fn koalabear_sp1_matches_sp1_vectors() {
        for (input, expected) in KOALABEAR_SP1_VECTORS {
            let mut state = input;
            POSEIDON2_KOALABEAR_SP1.permute(&mut state);
            assert_eq!(state, expected, "input {input:?}");
        }
    }

    /// Numbers between `start` (the first occurrence after `name`) and the next `end`.
    fn numbers(src: &str, name: &str, start: char, end: char) -> Vec<u64> {
        let from = src.find(name).unwrap_or_else(|| panic!("{name} not found"));
        let body = &src[from + name.len()..];
        let body = &body[body.find(start).unwrap() + 1..];
        body[..body.find(end).unwrap()]
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()))
            .map(|t| t.parse().unwrap())
            .collect()
    }

    /// The PIL and C++ copies of the constants are the ones of POSEIDON2_KOALABEAR_SP1.
    #[test]
    fn koalabear_sp1_constants_match_pil_and_cpp() {
        let kb = &POSEIDON2_KOALABEAR_SP1;
        let u = |v: &[u32]| v.iter().map(|&x| u64::from(x)).collect::<Vec<_>>();
        let m4 = u(kb.m4.as_flattened());
        let diag = u(&kb.internal_diag);
        let rc: Vec<u64> = [kb.rc_begin.as_flattened(), kb.rc_partial, kb.rc_end.as_flattened()]
            .iter()
            .flat_map(|v| u(v))
            .collect();

        let pil = include_str!("../../../poseidon/pil/poseidon2_koalabear_sp1.pil");
        assert_eq!(numbers(pil, "const int KB_SP1_P =", ' ', ';'), [u64::from(kb.p)]);
        assert_eq!(
            numbers(pil, "const int KB_SP1_INT_SCALE =", ' ', ';'),
            [u64::from(kb.internal_scale)]
        );
        assert_eq!(numbers(pil, "const int KB_SP1_M4[16] =", '[', ']'), m4);
        assert_eq!(numbers(pil, "const int KB_SP1_D_16[16] =", '[', ']'), diag);
        assert_eq!(numbers(pil, "const int KB_SP1_RC_16[148] =", '[', ']'), rc);

        let cpp = include_str!("../../../../lib-c/c/src/poseidon2/poseidon2_non_native.cpp");
        let params = numbers(cpp, "Poseidon2Params KOALABEAR_SP1 =", '{', '&');
        let [p, width, degree, m4_cpp @ .., scale] = params.as_slice() else { panic!() };
        assert_eq!(
            (*p, *width, *degree, *scale),
            (u64::from(kb.p), 16, u64::from(kb.sbox_degree), u64::from(kb.internal_scale))
        );
        assert_eq!(m4_cpp, m4);
        assert_eq!(numbers(cpp, "KB_SP1_INTERNAL_DIAG[16] =", '{', ';'), diag);
        let rc_cpp: Vec<u64> =
            ["KB_SP1_RC_BEGIN[4][16] =", "KB_SP1_RC_PARTIAL[20] =", "KB_SP1_RC_END[4][16] ="]
                .iter()
                .flat_map(|name| numbers(cpp, name, '{', ';'))
                .collect();
        assert_eq!(rc_cpp, rc);
    }

    #[test]
    fn first_noncanonical_finds_the_lane() {
        let p = POSEIDON2_KOALABEAR_SP1.p;
        let mut state = [p - 1; 16];
        assert_eq!(POSEIDON2_KOALABEAR_SP1.first_noncanonical(&state), None);
        state[5] = p;
        assert_eq!(POSEIDON2_KOALABEAR_SP1.first_noncanonical(&state), Some(5));
    }
}
