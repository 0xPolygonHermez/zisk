use ziskos::zisklib::U256;

pub const U256_MAX_MINUS_ONE: U256 = U256::from_u64s(&[u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX]);

pub const U256_MAX_MINUS_TWO: U256 = U256::from_u64s(&[u64::MAX - 2, u64::MAX, u64::MAX, u64::MAX]);

pub const U256_MAX_MINUS_THREE: U256 =
    U256::from_u64s(&[u64::MAX - 3, u64::MAX, u64::MAX, u64::MAX]);

pub const U256_MAX_HALF_PLUS_ONE: U256 = U256::from_u64s(&[1, 0, 0, 1 << 63]); // 2^255 + 1

pub const U256_MAX_HALF: U256 = U256::from_u64s(&[0, 0, 0, 1 << 63]); // 2^255

pub const U256_MAX_HALF_MINUS_ONE: U256 =
    U256::from_u64s(&[u64::MAX, u64::MAX, u64::MAX, (1 << 63) - 1]); // 2^255 - 1

pub const U256_MAX_QUARTER: U256 = U256::from_u64s(&[0, 0, 0, 1 << 62]); // 2^254
