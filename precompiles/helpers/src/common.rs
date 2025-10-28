use ark_bls12_381::Fq as Bls12_381Field;
use ark_bn254::Fq as Bn254Field;
use ark_ff::PrimeField;
use ark_secp256k1::Fq as Secp256k1Field;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;

pub fn bigint_to_16bit_chunks<const N: usize>(value: &BigInt, result: &mut [i64; N]) {
    assert!(N % 4 == 0, "chunk count N={} must be multiple of 4", N);
    let (sign, limbs) = value.to_u64_digits();
    let limbs_count = limbs.len();
    let limbs_needed = N / 4;
    #[allow(clippy::needless_range_loop)]
    for i in 0..limbs_needed {
        let mut limb64_value = if i < limbs_count { limbs[i] } else { 0 };
        for j in 0..4 {
            let idx = i * 4 + j;
            // last chunk has more than 16 bits to avoid an extra chunk.
            let limb16_value =
                if idx == N - 1 { limb64_value } else { limb64_value & 0xFFFF } as i64;
            result[idx] = if sign == Sign::Minus { -limb16_value } else { limb16_value };
            limb64_value >>= 16;
        }
    }
    if limbs_count > limbs_needed {
        assert_eq!(limbs_count, limbs_needed + 1);
        let chunk16_value = (limbs[limbs_needed] as i64) << 16;
        result[N - 1] += if sign == Sign::Minus { -chunk16_value } else { chunk16_value };
    }
}

#[inline]
pub fn bigint_to_16_chunks(value: &BigInt, result: &mut [i64; 16]) {
    bigint_to_16bit_chunks::<16>(value, result);
}

#[inline]
pub fn bigint_to_24_chunks(value: &BigInt, result: &mut [i64; 24]) {
    bigint_to_16bit_chunks::<24>(value, result);
}

pub fn bigint_from_u64s(words: &[u64]) -> BigInt {
    let mut result = BigInt::zero();

    for &word in words.iter().rev() {
        result <<= 64;
        result += word;
    }

    result
}

pub fn bigint_to_u64_limbs<const N: usize>(value: &BigInt, result: &mut [u64; N]) {
    let (sign, chunks) = value.to_u64_digits();
    assert!(
        sign == Sign::Plus || sign == Sign::NoSign,
        "bigint_to_u64_limbs: with negative value {value}"
    );
    let len = chunks.len();
    assert!(len <= N, "bigint_to_u64_limbs: value 0x{value:X} needs {len} limbs > {N}");
    for i in 0..N {
        result[i] = if i < len { chunks[i] } else { 0 };
    }
}

pub fn bigint_to_u64_limbs_with_cout<const N: usize>(value: &BigInt, result: &mut [u64; N]) -> u64 {
    let (sign, chunks) = value.to_u64_digits();
    assert!(
        sign == Sign::Plus || sign == Sign::NoSign,
        "bigint_to_u64_limbs: with negative value {value}"
    );

    let len = chunks.len();
    assert!(len <= (N + 1), "bigint_to_u64_limbs: value 0x{value:X} needs {len} limbs > {N}");
    for i in 0..N {
        result[i] = if i < len { chunks[i] } else { 0 };
    }
    if len == N + 1 {
        chunks[len - 1]
    } else {
        0
    }
}

#[inline]
pub fn bigint_to_4_u64(value: &BigInt, result: &mut [u64; 4]) {
    bigint_to_u64_limbs::<4>(value, result);
}

#[inline]
pub fn bigint_to_4_u64_with_cout(value: &BigInt, result: &mut [u64; 4]) -> u64 {
    bigint_to_u64_limbs_with_cout::<4>(value, result)
}

#[inline]
pub fn bigint_to_6_u64(value: &BigInt, result: &mut [u64; 6]) {
    bigint_to_u64_limbs::<6>(value, result);
}

pub fn bigint2_to_u64_limbs<const N: usize>(x: &BigInt, y: &BigInt, out: &mut [u64]) {
    assert!(out.len() == 2 * N, "expected out len {}, got {}", 2 * N, out.len());
    let (x_sign, x_chunks) = x.to_u64_digits();
    let (y_sign, y_chunks) = y.to_u64_digits();
    assert!(
        x_sign != Sign::Minus && y_sign != Sign::Minus,
        "bigint2_to_u64_limbs: with negative value x:{x} y:{y}"
    );
    let x_chunks_count = x_chunks.len();
    let y_chunks_count = y_chunks.len();
    assert!(x_chunks_count <= N, "x too large (needs {} limbs > N={})", x_chunks_count, N);
    assert!(y_chunks_count <= N, "y too large (needs {} limbs > N={})", y_chunks_count, N);
    for i in 0..N {
        out[i] = if i < x_chunks_count { x_chunks[i] } else { 0 };
        out[N + i] = if i < y_chunks_count { y_chunks[i] } else { 0 };
    }
}

#[inline]
pub fn bigint2_to_8_u64(x: &BigInt, y: &BigInt, result: &mut [u64; 8]) {
    bigint2_to_u64_limbs::<4>(x, y, result);
}

#[inline]
pub fn bigint2_to_12_u64(x: &BigInt, y: &BigInt, result: &mut [u64; 12]) {
    bigint2_to_u64_limbs::<6>(x, y, result);
}

pub fn bigint_to_2x4_u64(value: &BigInt, lres: &mut [u64; 4], hres: &mut [u64; 4]) {
    let (sign, chunks) = value.to_u64_digits();
    assert!(
        sign == Sign::Plus || sign == Sign::NoSign,
        "bigint_to_4_u64: with negative value {value}"
    );
    let chunks_count = chunks.len();
    assert!(chunks_count <= 8, "bigint_to_2x4_u64: with too big value 0x{value:X}");
    #[allow(clippy::needless_range_loop)]
    for i in 0..4 {
        if i >= chunks_count {
            lres[i] = 0;
            hres[i] = 0;
        } else {
            lres[i] = chunks[i];
            let hi = i + 4;
            hres[i] = if hi >= chunks_count { 0 } else { chunks[hi] };
        }
    }
}

pub trait FieldToBigInt {
    fn to_bigint(&self) -> BigInt;
}

impl FieldToBigInt for Secp256k1Field {
    fn to_bigint(&self) -> BigInt {
        let mut result = BigInt::zero();
        for &word in self.into_bigint().0.iter().rev() {
            result <<= 64;
            result += word;
        }
        result
    }
}

impl FieldToBigInt for Bn254Field {
    fn to_bigint(&self) -> BigInt {
        let mut result = BigInt::zero();
        for &word in self.into_bigint().0.iter().rev() {
            result <<= 64;
            result += word;
        }
        result
    }
}

impl FieldToBigInt for Bls12_381Field {
    fn to_bigint(&self) -> BigInt {
        let mut result = BigInt::zero();
        for &word in self.into_bigint().0.iter().rev() {
            result <<= 64;
            result += word;
        }
        result
    }
}

pub fn bigint_from_field<F: FieldToBigInt>(value: &F) -> BigInt {
    value.to_bigint()
}
