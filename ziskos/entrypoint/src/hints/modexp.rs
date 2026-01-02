use crate::hints::{
    HINT_QUEUE, check_main_thread,
    hint::Hint,
    types::{HINTS_TYPE_MODEXP, HintData}
};

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModExp {
    pub payload: Vec<u64>, // format: [base_len][base][exp_len][exp][modulus_len][modulus]
}

impl ModExp {
    pub fn new(base: Vec<u64>, exp: Vec<u64>, modulus: Vec<u64>) -> Self {
        let mut payload = Vec::with_capacity(base.len() + exp.len() + modulus.len());
        // Append base length and base
        payload.push(base.len() as u64);
        payload.append(&mut base.clone());
        // Append exponent length and exponent
        payload.push(exp.len() as u64);
        payload.append(&mut exp.clone());
        // Append modulus length and modulus
        payload.push(modulus.len() as u64);
        // Append modulus
        payload.append(&mut modulus.clone());


        Self {
            payload,
        }
    }
}

impl Default for ModExp {
    fn default() -> Self {
        Self {
            payload: Vec::new(),
        }
    }
}

impl HintData for ModExp {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let header_modexp: [u8; 8] =
            (((HINTS_TYPE_MODEXP as u64) << 32) | (self.payload.len() * core::mem::size_of::<u64>()) as u64).to_le_bytes();

        // Convert payload to bytes
        let payload_bytes = unsafe {
            core::slice::from_raw_parts(
                self.payload.as_ptr() as *const u8,
                self.payload.len() * core::mem::size_of::<u64>(),
            )
        };

        (header_modexp, payload_bytes)
    }
}

#[inline(always)]
pub fn hint_modexp(base: Vec<u64>, exp: Vec<u64>, modulus: Vec<u64>) {
    check_main_thread();

    let hint = Hint::ModExp(ModExp::new(base, exp, modulus));
    HINT_QUEUE.push(hint);
}