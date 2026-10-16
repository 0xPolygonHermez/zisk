//! Poseidon2 over 31-bit prime fields (KoalaBear, BabyBear, ...), on canonical `u32` lanes.
//!
//! One `Poseidon2Params` value describes an instance; the permutation is generic over it:
//! - external layer: `circ(2*M4, M4, ..., M4)` (Plonky3 layout), also applied before round 1;
//! - internal layer: `internal_scale * (J + diag(internal_diag))`;
//! - `rc_begin.len()` full rounds, `rc_partial.len()` partial rounds, `rc_end.len()` full rounds;
//! - in memory each lane x is stored as `io_scale * x`: `2^32 mod P` is the Montgomery form in
//!   which Plonky3 keeps KoalaBear and BabyBear, so guests pass their elements as they are.

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
    pub io_scale: u32,
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

    /// `1 / io_scale mod P`: the factor from the memory form of a lane to its canonical value.
    pub fn io_scale_inv(&self) -> u32 {
        pow_mod(self.io_scale, self.p - 2, self.p)
    }

    /// Permutes a state as it is in memory: two lanes per word (see `lanes`), each in the form
    /// `io_scale * x`. On a lane that is not canonical returns its index and leaves `words`
    /// untouched.
    pub fn permute_words(&self, words: &mut [u64]) -> Result<(), usize> {
        let lanes = Self::lanes(words);
        if let Some(i) = self.first_noncanonical(&lanes) {
            return Err(i);
        }
        let inv = self.io_scale_inv();
        let mut state = lanes.map(|m| self.mul(m, inv));
        self.permute(&mut state);
        for (word, pair) in words.iter_mut().zip(state.chunks_exact(2)) {
            let [lo, hi] = [pair[0], pair[1]].map(|y| self.mul(y, self.io_scale));
            *word = u64::from(lo) | (u64::from(hi) << 32);
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

pub(crate) fn pow_mod(base: u32, mut exp: u32, m: u32) -> u32 {
    let m = u64::from(m);
    let mut b = u64::from(base) % m;
    let mut result = 1u64;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * b % m;
        }
        b = b * b % m;
        exp >>= 1;
    }
    result as u32
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
        assert_eq!(numbers(pil, "const int KB_SP1_IO_SCALE =", ' ', ';'), [u64::from(kb.io_scale)]);
        assert_eq!(numbers(pil, "const int KB_SP1_M4[16] =", '[', ']'), m4);
        assert_eq!(numbers(pil, "const int KB_SP1_D_16[16] =", '[', ']'), diag);
        assert_eq!(numbers(pil, "const int KB_SP1_RC_16[148] =", '[', ']'), rc);

        let cpp = include_str!("../../../../lib-c/c/src/poseidon2/poseidon2_non_native.cpp");
        let params = numbers(cpp, "Poseidon2Params KOALABEAR_SP1 =", '{', '&');
        let [p, width, degree, m4_cpp @ .., scale, io_scale] = params.as_slice() else { panic!() };
        assert_eq!(
            (*p, *width, *degree, *scale, *io_scale),
            (
                u64::from(kb.p),
                16,
                u64::from(kb.sbox_degree),
                u64::from(kb.internal_scale),
                u64::from(kb.io_scale)
            )
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

    /// permute_words takes and returns the lanes in memory form (Montgomery for KoalaBear SP1).
    #[test]
    fn koalabear_sp1_permute_words_is_in_montgomery_form() {
        let kb = &POSEIDON2_KOALABEAR_SP1;
        let p = u64::from(kb.p);
        assert_eq!(u64::from(kb.io_scale), (1u64 << 32) % p);
        let mont = |x: u32| (u64::from(x) << 32) % p;
        for (input, expected) in KOALABEAR_SP1_VECTORS {
            let pack = |v: [u32; 16]| -> Vec<u64> {
                v.chunks_exact(2).map(|c| mont(c[0]) | (mont(c[1]) << 32)).collect()
            };
            let mut words = pack(input);
            kb.permute_words(&mut words).unwrap();
            assert_eq!(words, pack(expected));
        }
    }

    /// The C++ implementation (lib-c, run by the ASM emulator) gives the same words as
    /// permute_words on the SP1 vectors and on random and edge states, and rejects the same
    /// non-canonical states, leaving them untouched.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn koalabear_sp1_cpp_matches_rust() {
        let kb = &POSEIDON2_KOALABEAR_SP1;
        let p = kb.p;
        let check = |lanes: [u32; 16]| {
            let mut rust: [u64; 8] = std::array::from_fn(|w| {
                u64::from(lanes[2 * w]) | (u64::from(lanes[2 * w + 1]) << 32)
            });
            let mut cpp = rust;
            let accepted = kb.permute_words(&mut rust).is_ok();
            let rc = zisk_lib_c::poseidon2_koalabear_c(&mut cpp);
            assert_eq!(rc == 0, accepted, "return code {rc} on {lanes:x?}");
            assert_eq!(cpp, rust, "on {lanes:x?}");
        };

        let mont = |x: u32| ((u64::from(x) << 32) % u64::from(p)) as u32;
        for (input, _) in KOALABEAR_SP1_VECTORS {
            check(input.map(mont));
        }
        check([0; 16]);
        check([p - 1; 16]);

        // Deterministic pseudo-random states (splitmix64), with some 0 and P - 1 lanes
        let mut seed = 0x5eed_u64;
        let mut next = || {
            seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let z = (seed ^ (seed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            let z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        for _ in 0..10_000 {
            check(std::array::from_fn(|_| {
                let r = next();
                match r % 16 {
                    0 => 0,
                    1 => p - 1,
                    _ => (r >> 32) as u32 % p,
                }
            }));
        }

        // One lane not below P, in the low and the high half of a word
        for bad in [p, p + 1, u32::MAX] {
            for lane in [0, 7, 15] {
                let mut lanes = [1; 16];
                lanes[lane] = bad;
                check(lanes);
            }
        }
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
