use crate::hints::{
    HINT_QUEUE, check_main_thread,
    hint::Hint,
};

pub const HINTS_MODEXP: u32 = 13;

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HintModExp {
    pub payload: Vec<u64>, // format: [base_len][base][exp_len][exp][modulus_len][modulus]
}

impl HintModExp {
    pub fn new(base: Vec<u64>, exp: Vec<u64>, modulus: Vec<u64>) -> Self {
        let mut payload = Vec::with_capacity(3 + base.len() + exp.len() + modulus.len());
        // Append base length and base
        payload.push(base.len() as u64);
        payload.extend_from_slice(&base);
        // Append exponent length and exponent
        payload.push(exp.len() as u64);
        payload.extend_from_slice(&exp);
        // Append modulus length and modulus
        payload.push(modulus.len() as u64);
        // Append modulus
        payload.extend_from_slice(&modulus);

        Self {
            payload,
        }
    }
}

impl Default for HintModExp {
    fn default() -> Self {
        Self {
            payload: Vec::new(),
        }
    }
}

impl HintModExp {
    #[inline(always)]
    pub fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let header: [u8; 8] =
            (((HINTS_MODEXP as u64) << 32) | self.payload.len() as u64).to_le_bytes();

        // Convert payload to bytes
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self.payload.as_ptr() as *const u8,
                self.payload.len() * core::mem::size_of::<u64>(),
            )
        };

        (header, bytes)
    }

    #[inline(always)]
    pub fn hint_id(&self) -> u32 {
        HINTS_MODEXP
    }
}

#[inline(always)]
pub fn hint_modexp(base: Vec<u64>, exp: Vec<u64>, modulus: Vec<u64>) {
    if HINT_QUEUE.is_paused() {
        return;
    }

    check_main_thread();

    let hint = Hint::HintModExp(HintModExp::new(base, exp, modulus));
    HINT_QUEUE.push(hint);
}