use std::{collections::HashMap, mem, os::raw::c_void};

#[repr(C)]
pub struct StepsParams {
    pub buffer: *mut c_void,
    pub public_inputs: *mut c_void,
    pub challenges: *mut c_void,
    pub subproof_values: *mut c_void,
    pub evals: *mut c_void,
    pub xdivxsub: *mut c_void,
}

impl From<&StepsParams> for *mut c_void {
    fn from(params: &StepsParams) -> *mut c_void {
        params as *const StepsParams as *mut c_void
    }
}

/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
#[repr(C)]
pub struct AirInstance<F> {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub air_segment_id: Option<usize>,
    pub air_instance_id: Option<usize>,
    pub idx: Option<usize>,
    pub buffer: Vec<F>,
    pub subproof_values: Vec<F>,
    pub evals: Vec<F>,
    pub commits_calculated: HashMap<usize, bool>,
    pub subproofvalue_calculated: HashMap<usize, bool>,
}

impl<F> AirInstance<F> {
    pub fn new(airgroup_id: usize, air_id: usize, air_segment_id: Option<usize>, buffer: Vec<F>) -> Self {
        AirInstance {
            airgroup_id,
            air_id,
            air_segment_id,
            air_instance_id: None,
            idx: None,
            buffer,
            subproof_values: Vec::new(),
            evals: Vec::new(),
            commits_calculated: HashMap::new(),
            subproofvalue_calculated: HashMap::new(),
        }
    }

    pub fn get_buffer_ptr(&self) -> *mut u8 {
        self.buffer.as_ptr() as *mut u8
    }

    pub fn init_prover(&mut self, evals: Vec<F>, subproof_values: Vec<F>) {
        self.evals = evals;
        self.subproof_values = subproof_values;
    }

    pub fn set_commit_calculated(&mut self, id: usize) {
        self.commits_calculated.insert(id, true);
    }

    pub fn set_air_instance_id(&mut self, air_instance_id: usize, idx: usize) {
        self.air_instance_id = Some(air_instance_id);
        self.idx = Some(idx);
    }

    pub fn set_subproofvalue_calculated(&mut self, id: usize) {
        self.subproofvalue_calculated.insert(id, true);
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
