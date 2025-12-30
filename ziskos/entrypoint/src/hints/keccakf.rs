use crate::hints::{HINT_QUEUE, check_main_thread, hint::Hint, types::{HINTS_TYPE_RESULT, HintData}};

// KeccakF
pub const KECCAKF_LEN_U64: u64 = 25;
pub const KECCAKF_BYTES: usize = (KECCAKF_LEN_U64 as usize) * core::mem::size_of::<u64>();
pub const HEADER_KECCAKF: [u8; 8] =
    (((HINTS_TYPE_RESULT as u64) << 32) | KECCAKF_LEN_U64).to_le_bytes();

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct KeccakF {
    pub state: [u64; 25],
}

impl KeccakF {
    pub fn new(state: [u64; 25]) -> Self {
        Self { state }
    }
}

impl Default for KeccakF {
    fn default() -> Self {
        Self {
            state: [0u64; 25],
        }
    }
}

impl HintData for KeccakF {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let bytes = unsafe {
            core::slice::from_raw_parts(self.state.as_ptr() as *const u8, KECCAKF_BYTES)
        };
        (HEADER_KECCAKF, bytes)
    }
}

#[inline(always)]
pub fn hint_keccakf(state: &[u64; 25]) {
    check_main_thread();

    let hint = Hint::KeccakF(KeccakF::new(*state));
    HINT_QUEUE.push(hint);
}