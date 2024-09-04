use std::ffi::c_void;

use proofman_starks_lib_c::{
    transcript_add_c, transcript_add_polinomial_c, transcript_free_c, transcript_new_c, get_challenge_c,
    get_permutations_c,
};

pub struct FFITranscript {
    element_type: u32,
    p_stark: *mut c_void,
    pub p_transcript: *mut c_void,
}

impl FFITranscript {
    /// Creates a new transcript struct
    /// element_type: 0 for BN128, 1 for Goldilocks
    pub fn new(p_stark: *mut c_void, element_type: u32, arity: u64, custom: bool) -> Self {
        let p_transcript = transcript_new_c(element_type, arity, custom);

        Self { element_type, p_stark, p_transcript }
    }

    pub fn add_elements(&self, input: *mut c_void, size: usize) {
        transcript_add_c(self.p_transcript, input, size as u64);
    }

    pub fn add_polinomial(&self, p_polinomial: *mut c_void) {
        transcript_add_polinomial_c(self.p_transcript, p_polinomial);
    }

    pub fn get_challenge(&self, p_element: *mut c_void) {
        get_challenge_c(self.p_stark, self.p_transcript, p_element);
    }

    pub fn get_permutations(&self, res: *mut u64, n: u64, n_bits: u64) {
        get_permutations_c(self.p_transcript, res, n, n_bits);
    }

    /// Frees the memory of the transcript
    pub fn free(&self) {
        transcript_free_c(self.p_transcript, self.element_type);
    }
}
