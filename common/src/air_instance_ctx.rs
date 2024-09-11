use std::os::raw::c_void;

/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
pub struct AirInstanceCtx<F> {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub air_segment_id: Option<usize>,
    pub prover_idx: usize,
    pub buffer: Option<Vec<F>>,
    pub params: Option<*mut c_void>,
    pub commits_calculated: Vec<bool>,
    pub subproofvalue_calculated: Vec<bool>,
}

impl<F> AirInstanceCtx<F> {
    pub fn new(
        airgroup_id: usize,
        air_id: usize,
        air_segment_id: Option<usize>,
        prover_idx: usize,
        buffer: Option<Vec<F>>,
    ) -> Self {
        AirInstanceCtx {
            airgroup_id,
            air_id,
            air_segment_id,
            prover_idx,
            buffer,
            params: None,
            commits_calculated: Vec::new(),
            subproofvalue_calculated: Vec::new(),
        }
    }

    pub fn get_buffer_ptr(&mut self) -> *mut u8 {
        println!("airgroup_id: {}, Air_id: {}", self.airgroup_id, self.air_id);
        if self.buffer.is_some() {
            self.buffer.as_mut().unwrap().as_mut_ptr() as *mut u8
        } else {
            panic!("Buffer not initialized");
        }
    }

    pub fn set_params(&mut self, params: *mut c_void) {
        self.params = Some(params);
    }

    pub fn init_vec(&mut self, n_commits: usize, n_subproofvalues: usize) {
        self.commits_calculated = vec![false; n_commits];
        self.subproofvalue_calculated = vec![false; n_subproofvalues];
    }

    pub fn set_commit_calculated(&mut self, id: usize) {
        self.commits_calculated[id] = true;
    }

    pub fn set_subproofvalue_calculated(&mut self, id: usize) {
        self.subproofvalue_calculated[id] = true;
    }
}

unsafe impl<F> Send for AirInstanceCtx<F> {}
unsafe impl<F> Sync for AirInstanceCtx<F> {}
