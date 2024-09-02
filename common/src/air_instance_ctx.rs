use std::os::raw::c_void;

/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
pub struct AirInstanceCtx<F> {
    pub air_group_id: usize,
    pub air_id: usize,
    pub prover_idx: usize,
    pub buffer: Option<Vec<F>>,
    pub params: Option<*mut c_void>,
}

impl<F> AirInstanceCtx<F> {
    pub fn new(air_group_id: usize, air_id: usize, prover_idx: usize, buffer: Option<Vec<F>>) -> Self {
        AirInstanceCtx { air_group_id, air_id, prover_idx, buffer, params: None }
    }

    pub fn get_buffer_ptr(&mut self) -> *mut u8 {
        println!("Air_group_id: {}, Air_id: {}", self.air_group_id, self.air_id);
        if self.buffer.is_some() {
        self.buffer.as_mut().unwrap().as_mut_ptr() as *mut u8
    } else {
            panic!("Buffer not initialized");
        }
    }

    pub fn set_params(&mut self, params: *mut c_void) {
        self.params = Some(params);
    }
}
