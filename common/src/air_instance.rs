use std::mem;

/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
#[repr(C)]
pub struct AirInstance<F> {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub air_segment_id: Option<usize>,
    pub buffer: Vec<F>,
    pub subproof_values: Vec<F>,
    pub evals: Vec<F>,
    pub commits_calculated: Vec<bool>,
    pub subproofvalue_calculated: Vec<bool>,
}

impl<F> AirInstance<F> {
    pub fn new(airgroup_id: usize, air_id: usize, air_segment_id: Option<usize>, buffer: Vec<F>) -> Self {
        AirInstance {
            airgroup_id,
            air_id,
            air_segment_id,
            buffer,
            subproof_values: Vec::new(),
            evals: Vec::new(),
            commits_calculated: Vec::new(),
            subproofvalue_calculated: Vec::new(),
        }
    }

    pub fn get_buffer_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr() as *mut u8
    }

    pub fn init_prover(&mut self, n_commits: usize, n_subproofvalues: usize, evals: Vec<F>, subproof_values: Vec<F>) {
        self.evals = evals;
        self.subproof_values = subproof_values;

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

pub struct AirInstanceBuilder<F> {
    airgroup_id: usize,
    air_id: usize,
    air_segment_id: Option<usize>,
    buffer: Vec<F>,
}

#[allow(dead_code)]
impl<F> AirInstanceBuilder<F> {
    pub fn with_airgroup_id(&mut self, airgroup_id: usize) -> &mut Self {
        self.airgroup_id = airgroup_id;
        self
    }

    pub fn with_air_id(&mut self, air_id: usize) -> &mut Self {
        self.air_id = air_id;
        self
    }

    pub fn with_air_segment_id(&mut self, air_segment_id: Option<usize>) -> &mut Self {
        self.air_segment_id = air_segment_id;
        self
    }

    pub fn with_buffer(&mut self, buffer: Vec<F>) -> &mut Self {
        self.buffer = buffer;
        self
    }

    pub fn build(&mut self) -> AirInstance<F> {
        let buffer = mem::take(&mut self.buffer);
        AirInstance::new(self.airgroup_id, self.air_id, self.air_segment_id, buffer)
    }
}
