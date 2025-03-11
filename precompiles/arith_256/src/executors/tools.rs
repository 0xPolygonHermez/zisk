use ark_ff::PrimeField;
use ark_secp256k1::Fq as Secp256k1Field;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;

pub fn bigint_to_16_chunks(value: &BigInt, result: &mut [i64; 16]) {
    let (sign, chunks) = value.to_u64_digits();
    let chunks_count = chunks.len();
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
        result[15] += if sign == Sign::Minus { -chunk16_value } else { chunk16_value } as i64;
    }
}

pub fn bigints_from_u64s(words: &[u64]) -> Vec<BigInt> {
    words.chunks(4).map(|chunk| bigint_from_u64s(chunk)).collect()
}

pub fn bigint_from_u64s(words: &[u64]) -> BigInt {
    let mut result = BigInt::zero();

    for &word in words.iter().rev() {
        // Cada u64 representa un bloc de 64 bits, és a dir, 8 bytes.
        result <<= 64; // Shift per deixar lloc al següent word
        result += word;
    }

    result
}

pub fn bigint_from_field(value: &Secp256k1Field) -> BigInt {
    let mut result = BigInt::zero();

    for &word in value.into_bigint().0.iter().rev() {
        // Cada u64 representa un bloc de 64 bits, és a dir, 8 bytes.
        result <<= 64; // Shift per deixar lloc al següent word
        result += word;
    }

    result
}
