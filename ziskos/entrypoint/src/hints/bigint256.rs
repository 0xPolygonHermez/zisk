use crate::hints::{
    HINT_QUEUE, check_main_thread,
    hint::Hint,
    types::{HINTS_TYPE_ADDMOD256, HINTS_TYPE_DIVREM256, HINTS_TYPE_MULMOD256, HINTS_TYPE_OMUL256, HINTS_TYPE_REDMOD256, HINTS_TYPE_WMUL256, HINTS_TYPE_WPOW256, HintData},
    utils::{concat_2_u64x4, concat_3_u64x4}
};

// Uint256
pub const UINT256_LEN_U64: u64 = 4; // 256 bits
pub const UINT256_BYTES: usize = (UINT256_LEN_U64 as usize) * core::mem::size_of::<u64>();

// TODO: Check use of a macro to generate BigInt256 hints

// === BigInt two param struct ===
#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]

pub struct BigIntOpTwoParam {
    pub a: [u64; 4],
    pub b: [u64; 4],
}

impl BigIntOpTwoParam {
    pub fn new(a: [u64; 4], b: [u64; 4]) -> Self {
        Self { a, b }
    }

    fn payload (&self) -> &[u8] {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                concat_2_u64x4(&self.a, &self.b).as_ptr() as *const u8,
                2 * UINT256_BYTES,
            )
        };
        bytes
    }
}

impl Default for BigIntOpTwoParam {
    fn default() -> Self {
        Self {
            a: [0u64; 4],
            b: [0u64; 4],
        }
    }
}
// === BigInt three param struct ===
#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BigIntOpThreeParam {
    pub a: [u64; 4],
    pub b: [u64; 4],
    pub c: [u64; 4],
}

impl BigIntOpThreeParam {
    pub fn new(a: [u64; 4], b: [u64; 4], m: [u64; 4]) -> Self {
        Self { a, b, c: m }
    }

    fn payload (&self) -> &[u8] {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                concat_3_u64x4(&self.a, &self.b, &self.c).as_ptr() as *const u8,
                3 * UINT256_BYTES,
            )
        };
        bytes
    }
}

impl Default for BigIntOpThreeParam {
    fn default() -> Self {
        Self {
            a: [0u64; 4],
            b: [0u64; 4],
            c: [0u64; 4],
        }
    }
}

// === redmod256 (a, m) ===
pub const HEADER_REDMOD256: [u8; 8] =
    (((HINTS_TYPE_REDMOD256 as u64) << 32) | UINT256_LEN_U64 * 2).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedMod256(BigIntOpTwoParam);

impl RedMod256 {
    pub fn new(a: [u64; 4], m: [u64; 4]) -> Self {
        Self(BigIntOpTwoParam::new(a, m))
    }
}

impl HintData for RedMod256 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = self.0.payload();
        (HEADER_REDMOD256, bytes)
    }
}

#[inline(always)]
pub fn hint_redmod256(a: &[u64; 4], m: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::RedMod256(RedMod256::new(*a, *m));
    HINT_QUEUE.push(hint);
}

// === addmod256 (a, b, m) ===
pub const HEADER_ADDMOD256: [u8; 8] =
    (((HINTS_TYPE_ADDMOD256 as u64) << 32) | UINT256_LEN_U64 * 3).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddMod256(pub BigIntOpThreeParam);

impl AddMod256 {
    pub fn new(a: [u64; 4], b: [u64; 4], m: [u64; 4]) -> Self {
        Self(BigIntOpThreeParam::new(a, b, m))
    }
}

impl HintData for AddMod256 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = self.0.payload();
        (HEADER_ADDMOD256, bytes)
    }
}

#[inline(always)]
pub fn hint_addmod256(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::AddMod256(AddMod256::new(*a, *b, *m));
    HINT_QUEUE.push(hint);
}

// === mulmod256 (a, b, m) ===
pub const HEADER_MULMOD256: [u8; 8] =
    (((HINTS_TYPE_MULMOD256 as u64) << 32) | UINT256_LEN_U64 * 3).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MulMod256(pub BigIntOpThreeParam);

impl MulMod256 {
    pub fn new(a: [u64; 4], b: [u64; 4], m: [u64; 4]) -> Self {
        Self(BigIntOpThreeParam::new(a, b, m))
    }
}

impl HintData for MulMod256 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = self.0.payload();
        (HEADER_MULMOD256, bytes)
    }
}

#[inline(always)]
pub fn hint_mulmod256(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::MulMod256(MulMod256::new(*a, *b, *m));
    HINT_QUEUE.push(hint);
}

// === divrem256 (a, b, q, r) ===
pub const HEADER_DIVREM256: [u8; 8] =
    (((HINTS_TYPE_DIVREM256 as u64) << 32) | UINT256_LEN_U64 * 4).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivRem256(BigIntOpTwoParam);

impl DivRem256 {
    pub fn new(a: [u64; 4], b: [u64; 4]) -> Self {
        Self(BigIntOpTwoParam::new(a, b))
    }
}

impl HintData for DivRem256 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = self.0.payload();
        (HEADER_DIVREM256, bytes)
    }
}

#[inline(always)]
pub fn hint_divrem256(a: &[u64; 4], b: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::DivRem256(DivRem256::new(*a, *b));
    HINT_QUEUE.push(hint);
}

// === wpow256 (a, exp) ===
pub const HEADER_WPOW256: [u8; 8] =
    (((HINTS_TYPE_WPOW256 as u64) << 32) | UINT256_LEN_U64 * 2).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WPow256(BigIntOpTwoParam);

impl WPow256 {
    pub fn new(a: [u64; 4], exp: [u64; 4]) -> Self {
        Self(BigIntOpTwoParam::new(a, exp))
    }
}

impl HintData for WPow256 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = self.0.payload();
        (HEADER_WPOW256, bytes)
    }
}

#[inline(always)]
pub fn hint_wpow256(a: &[u64; 4], exp: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::WPow256(WPow256::new(*a, *exp));
    HINT_QUEUE.push(hint);
}

// === omul256 (a, b) ===
pub const HEADER_OMUL256: [u8; 8] =
    (((HINTS_TYPE_OMUL256 as u64) << 32) | UINT256_LEN_U64 * 2).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OMul256(BigIntOpTwoParam);

impl OMul256 {
    pub fn new(a: [u64; 4], b: [u64; 4]) -> Self {
        Self(BigIntOpTwoParam::new(a, b))
    }
}

impl HintData for OMul256 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = self.0.payload();
        (HEADER_OMUL256, bytes)
    }
}

#[inline(always)]
pub fn hint_omul256(a: &[u64; 4], b: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::OMul256(OMul256::new(*a, *b));
    HINT_QUEUE.push(hint);
}

// === wmul256 (a, b) ===
pub const HEADER_WMUL256: [u8; 8] =
    (((HINTS_TYPE_WMUL256 as u64) << 32) | UINT256_LEN_U64 * 2).to_le_bytes();

 #[repr(C, align(8))]
 #[derive(Clone, Debug, Eq, PartialEq)]
 pub struct WMul256(BigIntOpTwoParam);

impl WMul256 {
    pub fn new(a: [u64; 4], b: [u64; 4]) -> Self {
        Self(BigIntOpTwoParam::new(a, b))
    }
}

impl HintData for WMul256 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = self.0.payload();
        (HEADER_WMUL256, bytes)
    }
}

#[inline(always)]
pub fn hint_wmul256(a: &[u64; 4], b: &[u64; 4]) {
    check_main_thread();

    let hint = Hint::WMul256(WMul256::new(*a, *b));
    HINT_QUEUE.push(hint);
}