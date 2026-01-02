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
    pub pk: [u8; 40],
    pub z: [u8; 32],
    pub sig: [u8; 64],
}

impl ECRecover {
    pub fn new(pk: [u8; 40], z: [u8; 32], sig: [u8; 64]) -> Self {
        Self { pk, z, sig }
    }
}

impl Default for ECRecover {
    fn default() -> Self {
        Self {
            pk: [0u8; 40],
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
pub fn hint_ecrecover(pk: &[u8; 33], z: &[u8; 32], sig: &[u8; 64]) {
    check_main_thread();

    let mut pk_aligned: [u8; 40] = [0u8; 40];
    pk_aligned[..33].copy_from_slice(pk);

    let hint = Hint::ECRecover(ECRecover::new(pk_aligned, *z, *sig));
    HINT_QUEUE.push(hint);
}