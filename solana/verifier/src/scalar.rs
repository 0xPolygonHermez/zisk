use ark_ff::{BigInt, BigInteger};
use core::ops::{Add, AddAssign, Mul, Neg};

const MASK: u64 = (1 << 29) - 1;
const INV: u64 = 0x0fff_ffff;
const MODULUS: BigInt<4> =
    BigInt([4891460686036598785, 2896914383306846353, 13281191951274694749, 3486998266802970665]);
const N: [u32; 9] =
    [268435457, 521120927, 240919632, 131109107, 361091715, 47923392, 10936641, 240920116, 3171406];
const R2: Fr = Fr([
    95853524, 102173274, 34397646, 498479371, 240439551, 486036963, 471195907, 131109217, 656714,
]);

/// Private canonical Montgomery residue; public proof bytes never use this representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fr(pub(crate) [u32; 9]);
impl Fr {
    pub(crate) const ZERO: Self = Self([0; 9]);
    pub(crate) const ONE: Self = Self([
        268435287, 514263732, 86771339, 391139145, 178784091, 490881230, 299191303, 86689704,
        903222,
    ]);
}

fn reduce(mut a: [u32; 9]) -> Fr {
    if a.iter().rev().cmp(N.iter().rev()) != core::cmp::Ordering::Less {
        let mut borrow = 0i64;
        for i in 0..9 {
            let value = i64::from(a[i]) - i64::from(N[i]) - borrow;
            a[i] = (value as u64 & MASK) as u32;
            borrow = i64::from(value < 0);
        }
    }
    Fr(a)
}

impl Add for Fr {
    type Output = Self;
    #[inline(never)]
    #[allow(clippy::needless_range_loop)] // Preserve the benchmarked SBF instruction sequence.
    fn add(self, rhs: Self) -> Self {
        let mut limbs = [0; 9];
        let mut carry = 0u64;
        for i in 0..9 {
            let value = u64::from(self.0[i]) + u64::from(rhs.0[i]) + carry;
            limbs[i] = (value & MASK) as u32;
            carry = value >> 29;
        }
        reduce(limbs)
    }
}
impl AddAssign for Fr {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl Neg for Fr {
    type Output = Self;
    fn neg(self) -> Self {
        if self == Self::ZERO {
            return self;
        }
        let mut out = [0; 9];
        let mut borrow = 0i64;
        for i in 0..9 {
            let value = i64::from(N[i]) - i64::from(self.0[i]) - borrow;
            out[i] = (value as u64 & MASK) as u32;
            borrow = i64::from(value < 0);
        }
        Self(out)
    }
}
impl Mul for Fr {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        mul(self, rhs)
    }
}
impl From<u64> for Fr {
    fn from(n: u64) -> Self {
        canonical(BigInt([n, 0, 0, 0])).unwrap()
    }
}

// Fixed indices avoid bounds checks and LLVM's variable loop-unrolling decisions on SBF.
macro_rules! unroll9 {
    ($i:ident, $body:block) => {{
        let $i = 0;
        $body
    }
    {
        let $i = 1;
        $body
    }
    {
        let $i = 2;
        $body
    }
    {
        let $i = 3;
        $body
    }
    {
        let $i = 4;
        $body
    }
    {
        let $i = 5;
        $body
    }
    {
        let $i = 6;
        $body
    }
    {
        let $i = 7;
        $body
    }
    {
        let $i = 8;
        $body
    }};
}

// Nine radix-2^29 Montgomery steps, R=2^261. Each column is bounded by
// 18*(2^29-1)^2 + carry < 2^63; a,b<r implies the result is <2r.
#[inline(always)]
fn montgomery(a: Fr, b: Fr) -> Fr {
    let mut m = [0u64; 9];
    let mut out = [0u32; 9];
    let mut carry = 0u64;
    macro_rules! column {
        ($k:expr) => {{
            let k: usize = $k;
            let mut acc = carry;
            unroll9!(i, {
                if k >= i && k - i < 9 {
                    acc = acc.wrapping_add(u64::from(a.0[i]).wrapping_mul(u64::from(b.0[k - i])));
                }
                if k >= i && k - i < 9 && i < k {
                    acc = acc.wrapping_add(m[i].wrapping_mul(u64::from(N[k - i])));
                }
            });
            if k < 9 {
                m[k] = acc.wrapping_mul(INV) & MASK;
                acc = acc.wrapping_add(m[k].wrapping_mul(u64::from(N[0])));
            }
            if k >= 9 {
                out[k - 9] = (acc & MASK) as u32;
            }
            carry = acc >> 29;
        }};
    }
    column!(0);
    column!(1);
    column!(2);
    column!(3);
    column!(4);
    column!(5);
    column!(6);
    column!(7);
    column!(8);
    column!(9);
    column!(10);
    column!(11);
    column!(12);
    column!(13);
    column!(14);
    column!(15);
    column!(16);
    out[8] = carry as u32;
    reduce(out)
}

#[inline(never)]
pub(crate) fn mul(a: Fr, b: Fr) -> Fr {
    montgomery(a, b)
}

#[inline(never)]
#[allow(clippy::needless_range_loop)] // Preserve the benchmarked SBF instruction sequence.
pub(crate) fn sub(a: Fr, b: Fr) -> Fr {
    let mut out = [0; 9];
    let mut borrow = 0i64;
    for i in 0..9 {
        let value = i64::from(a.0[i]) - i64::from(b.0[i]) - borrow;
        out[i] = (value as u64 & MASK) as u32;
        borrow = i64::from(value < 0);
    }
    if borrow != 0 {
        let mut carry = 0u64;
        for i in 0..9 {
            let value = u64::from(out[i]) + u64::from(N[i]) + carry;
            out[i] = (value & MASK) as u32;
            carry = value >> 29;
        }
    }
    Fr(out)
}

#[inline(never)]
pub(crate) fn square(a: Fr) -> Fr {
    montgomery(a, a)
}

pub(crate) fn pow(a: Fr, exponent: u64) -> Fr {
    if exponent == 0 {
        return Fr::ONE;
    }
    let mut result = a;
    for bit in (0..63 - exponent.leading_zeros()).rev() {
        result = square(result);
        if exponent & (1 << bit) != 0 {
            result = mul(result, a);
        }
    }
    result
}

#[inline(never)]
pub(crate) fn canonical(value: BigInt<4>) -> Option<Fr> {
    if value >= MODULUS {
        return None;
    }
    let mut out = [0; 9];
    for (i, limb) in out.iter_mut().enumerate() {
        let word = i * 29 / 64;
        let shift = i * 29 % 64;
        let mut v = value.0[word] >> shift;
        if word < 3 && shift > 35 {
            v |= value.0[word + 1] << (64 - shift);
        }
        *limb = (v & MASK) as u32;
    }
    Some(montgomery(Fr(out), R2))
}

pub(crate) fn hash(bytes: [u8; 32]) -> Fr {
    let mut value = BigInt(core::array::from_fn(|i| {
        u64::from_be_bytes(bytes[(3 - i) * 8..(4 - i) * 8].try_into().unwrap())
    }));
    while value >= MODULUS {
        value.sub_with_borrow(&MODULUS);
    }
    canonical(value).unwrap()
}

#[inline(never)]
pub(crate) fn integer(value: Fr) -> BigInt<4> {
    let mut one = [0; 9];
    one[0] = 1;
    let value = montgomery(value, Fr(one));
    let mut out = [0u64; 4];
    for i in 0..9 {
        let word = i * 29 / 64;
        let shift = i * 29 % 64;
        out[word] |= u64::from(value.0[i]) << shift;
        if word < 3 && shift > 35 {
            out[word + 1] |= u64::from(value.0[i]) >> (64 - shift);
        }
    }
    BigInt(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr as Ark;
    use ark_ff::{Field, PrimeField};
    fn ours(a: Ark) -> Fr {
        canonical(a.into_bigint()).unwrap()
    }
    fn check(a: Ark, b: Ark) {
        let x = ours(a);
        let y = ours(b);
        assert_eq!(integer(x), a.into_bigint());
        assert_eq!(integer(x + y), (a + b).into_bigint());
        assert_eq!(integer(-x), (-a).into_bigint());
        assert_eq!(integer(sub(x, y)), (a - b).into_bigint());
        assert_eq!(integer(mul(x, y)), (a * b).into_bigint());
        assert_eq!(integer(square(x)), a.square().into_bigint());
        assert!(mul(x, y).0.iter().all(|v| u64::from(*v) <= MASK));
    }
    #[test]
    fn conversions_powers_and_raw_limb_boundaries() {
        assert_eq!(canonical(MODULUS), None);
        for bytes in [[0; 32], [255; 32], MODULUS.to_bytes_be().try_into().unwrap()] {
            assert_eq!(integer(hash(bytes)), Ark::from_be_bytes_mod_order(&bytes).into_bigint());
        }
        let inv_r = Ark::from(2u64).pow([261u64]).inverse().unwrap();
        for bit in 0..254 {
            for delta in [-Ark::ONE, Ark::ZERO, Ark::ONE] {
                let a = (Ark::from(2u64).pow([bit]) + delta) * inv_r;
                check(a, -a);
                check(a, -inv_r);
            }
        }
        for exponent in [0, 1, 2, 16, 215, 216, 54288, u64::MAX] {
            for a in [Ark::ZERO, Ark::ONE, -Ark::ONE, Ark::from(7u64)] {
                assert_eq!(integer(pow(ours(a), exponent)), a.pow([exponent]).into_bigint());
            }
        }
    }

    #[test]
    fn arithmetic_matches_arkworks_edges_and_seeded_pairs() {
        assert_eq!(MODULUS, Ark::MODULUS);
        assert_eq!((u64::from(N[0]) * INV) & MASK, MASK);
        assert_eq!(integer(Fr::ONE), Ark::ONE.into_bigint());
        assert_eq!(canonical(MODULUS), None);
        let edges = [Ark::ZERO, Ark::ONE, -Ark::ONE, Ark::from(2u64), Ark::from(u64::MAX)];
        for a in edges {
            for b in edges {
                check(a, b);
            }
        }
        let mut seed = 0x7b5d_128e_e7c0_2451u64;
        for _ in 0..100_000 {
            let mut bytes = [0u8; 64];
            for part in bytes.chunks_exact_mut(8) {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                part.copy_from_slice(&seed.to_le_bytes());
            }
            let a = Ark::from_le_bytes_mod_order(&bytes[..32]);
            let b = Ark::from_le_bytes_mod_order(&bytes[32..]);
            check(a, b);
            assert_eq!(
                integer(hash(bytes[..32].try_into().unwrap())),
                Ark::from_be_bytes_mod_order(&bytes[..32]).into_bigint()
            );
        }
    }
}
