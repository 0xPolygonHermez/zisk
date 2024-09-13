/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
#[repr(C)]
pub struct AirInstanceCtx<F> {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub air_segment_id: Option<usize>,
    pub prover_idx: usize,
    pub buffer: Vec<F>,
    pub subproof_values: Vec<F>,
    pub evals: Vec<F>,
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
            buffer: buffer.unwrap(),
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
