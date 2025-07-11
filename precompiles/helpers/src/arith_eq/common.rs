use ark_bn254::Fq as Bn254Field;
use ark_ff::PrimeField;
use ark_secp256k1::Fq as Secp256k1Field;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;

pub fn bigint_to_16_chunks(value: &BigInt, result: &mut [i64; 16]) {
    let (sign, chunks) = value.to_u64_digits();
    let chunks_count = chunks.len();
    #[allow(clippy::needless_range_loop)]
    for i in 0..4 {
        let mut chunk64_value = if i >= chunks_count { 0 } else { chunks[i] };
        for j in 0..4 {
            let chunk_index = i * 4 + j;
            // last chunk has more than 16 bits to avoid an extra chunk.
            let chunk16_value =
                if chunk_index == 15 { chunk64_value } else { chunk64_value & 0xFFFF } as i64;
            result[chunk_index] = if sign == Sign::Minus { -chunk16_value } else { chunk16_value };
            chunk64_value >>= 16;
        }
    }
    if chunks_count > 4 {
        assert_eq!(chunks_count, 5);
        let chunk16_value = (chunks[4] as i64) << 16;
        result[15] += if sign == Sign::Minus { -chunk16_value } else { chunk16_value };
    }
}

pub fn bigint_from_u64s(words: &[u64]) -> BigInt {
    let mut result = BigInt::zero();

    for &word in words.iter().rev() {
        result <<= 64;
        result += word;
    }

    result
}

pub fn bigint_to_4_u64(value: &BigInt, result: &mut [u64; 4]) {
    let (sign, chunks) = value.to_u64_digits();
    assert!(
        sign == Sign::Plus || sign == Sign::NoSign,
        "bigint_to_4_u64: with negative value {value}"
    );
    let chunks_count = chunks.len();
    assert!(chunks_count <= 4, "bigint_to_4_u64: with too big value 0x{value:X}");
    #[allow(clippy::needless_range_loop)]
    for i in 0..4 {
        result[i] = if i >= chunks_count { 0 } else { chunks[i] };
    }
}

pub fn bigint2_to_8_u64(x: &BigInt, y: &BigInt, result: &mut [u64; 8]) {
    let (x_sign, x_chunks) = x.to_u64_digits();
    let (y_sign, y_chunks) = y.to_u64_digits();
    assert!(
        x_sign != Sign::Minus && y_sign != Sign::Minus,
        "bigint2_to_8_u64: with negative value x:{x} y:{y}"
    );
    let x_chunks_count = x_chunks.len();
    let y_chunks_count = y_chunks.len();
    assert!(x_chunks_count <= 4, "bigint_to_4_u64: with too big value x:0x{x:X}");
    assert!(y_chunks_count <= 4, "bigint_to_4_u64: with too big value y:0x{y:X}");
    #[allow(clippy::needless_range_loop)]
    for i in 0..4 {
        result[i] = if i >= x_chunks_count { 0 } else { x_chunks[i] };
        result[4 + i] = if i >= y_chunks_count { 0 } else { y_chunks[i] };
    }
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

pub fn bigint_from_field<F: FieldToBigInt>(value: &F) -> BigInt {
    value.to_bigint()
}
