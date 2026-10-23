//! Host-side check of `zisklib::modexp` against num-bigint's `modpow`, limb for limb, through
//! the one-limb (short) and multi-limb (long) modulus paths.

use num_bigint::BigUint;
use ziskos::zisklib::{modexp, U256};

/// A splitmix64 stream, so the cases are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// `words` random words whose most significant one is not zero.
    fn words(&mut self, words: usize) -> Vec<u64> {
        let mut out: Vec<u64> = (0..words).map(|_| self.next()).collect();
        while out[words - 1] == 0 {
            out[words - 1] = self.next();
        }
        out
    }
}

fn to_biguint(words: &[u64]) -> BigUint {
    BigUint::from_bytes_le(&words.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>())
}

/// `x` as U256 limbs without leading zero limbs (a single zero limb for zero), the form in
/// which `modexp` takes its operands and returns its result.
fn to_limbs(x: &BigUint) -> Vec<U256> {
    let mut words = x.to_u64_digits();
    words.resize(words.len().next_multiple_of(4).max(4), 0);
    words.chunks(4).map(|c| U256::from_u64s(&[c[0], c[1], c[2], c[3]])).collect()
}

/// `modexp(base, exp, modulus)` must equal num-bigint's result, limb for limb.
fn check(base: &BigUint, exp: &BigUint, modulus: &BigUint) {
    let mut exp_words = exp.to_u64_digits();
    if exp_words.is_empty() {
        exp_words.push(0);
    }
    let expected = to_limbs(&base.modpow(exp, modulus));
    let result = modexp(&to_limbs(base), &exp_words, &to_limbs(modulus));
    assert_eq!(result, expected, "base {base:#x}, exp {exp:#x}, modulus {modulus:#x}");
}

#[test]
fn modexp_matches_num_bigint() {
    let mut rng = Rng(1);
    for modulus_words in [1, 4, 5, 8, 9, 13, 16, 17, 32] {
        for exp_words in [1, 2, 5, 16] {
            for base_words in [1, modulus_words, modulus_words + 3] {
                for _ in 0..4 {
                    let modulus = to_biguint(&rng.words(modulus_words));
                    let base = to_biguint(&rng.words(base_words));
                    check(&base, &to_biguint(&rng.words(exp_words)), &modulus);
                }
            }
        }
    }
}

#[test]
fn modexp_matches_num_bigint_on_edge_cases() {
    let one = BigUint::from(1u32);
    let two_to = |bits: u32| BigUint::from(1u32) << bits;
    let moduli = [
        one.clone(),
        BigUint::from(7u32),
        two_to(255) + 19u32,
        two_to(256),
        two_to(256) + 3u32,
        two_to(512) - 1u32,
        two_to(700) + two_to(300),
    ];
    for modulus in &moduli {
        let bases = [
            BigUint::from(0u32),
            one.clone(),
            BigUint::from(2u32),
            two_to(128),
            modulus - 1u32,
            modulus.clone(),
            modulus + 1u32,
            modulus * 3u32,
            modulus * modulus - 1u32,
        ];
        let exps = [
            BigUint::from(0u32),
            one.clone(),
            BigUint::from(2u32),
            BigUint::from(300u32),
            two_to(64) - 1u32,
            two_to(1000),
            two_to(1000) - 1u32,
        ];
        for base in &bases {
            for exp in &exps {
                check(base, exp, modulus);
            }
        }
    }
}
