pub fn i64_to_u64_field(value: i64) -> u64 {
    const PRIME_MINUS_ONE: u64 = 0xFFFF_FFFF_0000_0000;
    if value >= 0 {
        value as u64
    } else {
        PRIME_MINUS_ONE - (0xFFFF_FFFF_FFFF_FFFF - value as u64)
    }
}
