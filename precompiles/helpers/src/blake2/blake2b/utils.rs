pub const fn bits_from_u64(value: u64) -> [u64; 64] {
    let mut bits = [0u64; 64];
    let mut i = 0;
    while i < 64 {
        bits[i] = (value >> i) & 1;
        i += 1;
    }
    bits
}

pub fn u64_from_bits(bits: &[u64; 64]) -> u64 {
    let mut value = 0u64;
    for (i, &bit) in bits.iter().enumerate() {
        if bit == 1 {
            value |= 1u64 << i;
        }
    }
    value
}
