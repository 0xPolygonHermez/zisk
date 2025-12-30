use crate::hints::{
    HINT_QUEUE, check_main_thread,
    hint::Hint,
    types::{HINTS_TYPE_ADDMOD256, HINTS_TYPE_MULMOD256, HINTS_TYPE_REDMOD256, HintData},
    utils::{concat_2_u64x4, concat_3_u64x4}
};

// Uint256
pub const UINT256_LEN_U64: u64 = 4; // 256 bits
pub const UINT256_BYTES: usize = (UINT256_LEN_U64 as usize) * core::mem::size_of::<u64>();

// redmod256 (a, m)
pub const HEADER_REDMOD256: [u8; 8] =
    (((HINTS_TYPE_REDMOD256 as u64) << 32) | UINT256_LEN_U64 * 2).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RedMod256 {
    pub a: [u64; 4],
    pub m: [u64; 4],
}

impl RedMod256 {
    pub fn new(a: [u64; 4], m: [u64; 4]) -> Self {
        Self { a, m }
    }
}

impl Default for RedMod256 {
    fn default() -> Self {
        Self {
            a: [0u64; 4],
            m: [0u64; 4],
        }
    }
}

impl HintData for RedMod256 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = unsafe {
            core::slice::from_raw_parts(concat_2_u64x4(&self.a, &self.m).as_ptr() as *const u8, 2 * UINT256_BYTES)
        };
        (HEADER_REDMOD256, bytes)
    }
}

#[inline(always)]
pub fn hint_redmod256(a: &[u64; 4], m: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::RedMod256(RedMod256::new(*a, *m));
    HINT_QUEUE.push(hint);
}

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ThreeParam256 {
    pub a: [u64; 4],
    pub b: [u64; 4],
    pub m: [u64; 4],
}

impl ThreeParam256 {
    pub fn new(a: [u64; 4], b: [u64; 4], m: [u64; 4]) -> Self {
        Self { a, b, m }
    }
}

impl Default for ThreeParam256 {
    fn default() -> Self {
        Self {
            a: [0u64; 4],
            b: [0u64; 4],
            m: [0u64; 4],
        }
    }
}

trait ThreeParamHintData {
    const HEADER: [u8; 8];

    fn value(&self) -> &ThreeParam256;
}

impl<T: ThreeParamHintData> HintData for T {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let data = self.value();
        let bytes = unsafe {
            core::slice::from_raw_parts(
                concat_3_u64x4(&data.a, &data.b, &data.m).as_ptr() as *const u8,
                3 * UINT256_BYTES,
            )
        };
        (T::HEADER, bytes)
    }
}

// addmod256 (a, b, m)
pub const HEADER_ADDMOD256: [u8; 8] =
    (((HINTS_TYPE_ADDMOD256 as u64) << 32) | UINT256_LEN_U64 * 3).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AddMod256(pub ThreeParam256);

impl AddMod256 {
    pub fn new(a: [u64; 4], b: [u64; 4], m: [u64; 4]) -> Self {
        Self(ThreeParam256::new(a, b, m))
    }
}

impl ThreeParamHintData for AddMod256 {
    const HEADER: [u8; 8] = HEADER_ADDMOD256;

    #[inline(always)]
    fn value(&self) -> &ThreeParam256 {
        &self.0
    }
}

#[inline(always)]
pub fn hint_addmod256(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::AddMod256(AddMod256::new(*a, *b, *m));
    HINT_QUEUE.push(hint);
}

// mulmod256 (a, b, m)
pub const HEADER_MULMOD256: [u8; 8] =
    (((HINTS_TYPE_MULMOD256 as u64) << 32) | UINT256_LEN_U64 * 3).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MulMod256(pub ThreeParam256);

impl MulMod256 {
    pub fn new(a: [u64; 4], b: [u64; 4], m: [u64; 4]) -> Self {
        Self(ThreeParam256::new(a, b, m))
    }
}

impl ThreeParamHintData for MulMod256 {
    const HEADER: [u8; 8] = HEADER_MULMOD256;

    #[inline(always)]
    fn value(&self) -> &ThreeParam256 {
        &self.0
    }
}

#[inline(always)]
pub fn hint_mulmod256(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::MulMod256(MulMod256::new(*a, *b, *m));
    HINT_QUEUE.push(hint);
}
