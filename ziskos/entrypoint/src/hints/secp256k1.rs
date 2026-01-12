use crate::hints::{
    HINT_QUEUE, check_main_thread,
    hint::Hint,
    types::{HINTS_TYPE_ECRECOVER, HintData}
};

// ECRecover
pub const ECRECOVER_BYTES: usize = core::mem::size_of::<ECRecover>();
const _: () = {
    if ECRECOVER_BYTES % 8 != 0 {
        panic!("ECRecover size must be multiple of 8");
    }
};
pub const ECRECOVER_LEN_U64: u64 = (ECRECOVER_BYTES as u64) / 8;
pub const HEADER_ECRECOVER: [u8; 8] =
    (((HINTS_TYPE_ECRECOVER as u64) << 32) | ECRECOVER_LEN_U64).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ECRecover {
    pub x: [u8; 32],
    pub y: [u8; 32],
    pub infinity: u64,
    pub z: [u8; 32],
    pub sig: [u8; 64],
}

impl ECRecover {
    pub fn new(x: [u8; 32], y: [u8; 32], infinity: u64, z: [u8; 32], sig: [u8; 64]) -> Self {
        Self { x, y, infinity, z, sig }
    }
}

impl Default for ECRecover {
    fn default() -> Self {
        Self {
            x: [0u8; 32],
            y: [0u8; 32],
            infinity: 0u64,
            z: [0u8; 32],
            sig: [0u8; 64],
        }
    }
}

impl HintData for ECRecover {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (self as *const ECRecover).cast::<u8>(),
                ECRECOVER_BYTES,
            )
        };
        (HEADER_ECRECOVER, bytes)
    }
}

#[inline(always)]
pub fn hint_ecrecover(x: &[u8; 32], y: &[u8; 32], infinity: u64, z: &[u8; 32], sig: &[u8; 64]) {
    check_main_thread();

    let hint = Hint::ECRecover(ECRecover::new(*x, *y, infinity, *z, *sig));
    HINT_QUEUE.push(hint);
}