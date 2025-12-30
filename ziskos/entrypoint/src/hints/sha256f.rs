use crate::hints::{
    HINT_QUEUE, check_main_thread,
    hint::Hint,
    types::{HINTS_TYPE_RESULT, HintData}
};

// SHA2
pub const SHA2_LEN_U64: u64 = 4;
pub const SHA2_BYTES: usize = core::mem::size_of::<[u32; 8]>();
pub const HEADER_SHA2: [u8; 8] =
    (((HINTS_TYPE_RESULT as u64) << 32) | SHA2_LEN_U64).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Sha2 {
    pub state: [u32; 8],
}

impl Sha2 {
    pub fn new(state: [u32; 8]) -> Self {
        Self { state }
    }
}

impl Default for Sha2 {
    fn default() -> Self {
        Self {
            state: [0u32; 8],
        }
    }
}

impl HintData for Sha2 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = unsafe {
            core::slice::from_raw_parts(self.state.as_ptr() as *const u8, SHA2_BYTES)
        };
        (HEADER_SHA2, bytes)
    }
}

#[inline(always)]
pub fn hint_sha2(state: &[u32; 8]) {
    check_main_thread();

    let hint = Hint::SHA2(Sha2::new(*state));
    HINT_QUEUE.push(hint);
}
