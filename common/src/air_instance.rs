use std::ptr;
use p3_field::Field;
use proofman_util::create_buffer_fast;

use crate::{trace::Trace, trace::Values};

#[repr(C)]
pub struct StepsParams {
    pub trace: *mut u8,
    pub aux_trace: *mut u8,
    pub public_inputs: *mut u8,
    pub proof_values: *mut u8,
    pub challenges: *mut u8,
    pub airgroup_values: *mut u8,
    pub airvalues: *mut u8,
    pub evals: *mut u8,
    pub xdivxsub: *mut u8,
    pub p_const_pols: *mut u8,
    pub p_const_tree: *mut u8,
    pub custom_commits: [*mut u8; 10],
    pub custom_commits_extended: [*mut u8; 10],
}

impl From<&StepsParams> for *mut u8 {
    fn from(params: &StepsParams) -> *mut u8 {
        params as *const StepsParams as *mut u8
    }
}

impl Default for StepsParams {
    fn default() -> Self {
        StepsParams {
            trace: ptr::null_mut(),
            aux_trace: ptr::null_mut(),
            public_inputs: ptr::null_mut(),
            proof_values: ptr::null_mut(),
            challenges: ptr::null_mut(),
            airgroup_values: ptr::null_mut(),
            airvalues: ptr::null_mut(),
            evals: ptr::null_mut(),
            xdivxsub: ptr::null_mut(),
            p_const_pols: ptr::null_mut(),
            p_const_tree: ptr::null_mut(),
            custom_commits: [ptr::null_mut(); 10],
            custom_commits_extended: [ptr::null_mut(); 10],
        }
    }
}

pub struct CustomCommitInfo<F> {
    pub trace: Vec<F>,
    pub commit_id: usize,
}

pub struct TraceInfo<F> {
    airgroup_id: usize,
    air_id: usize,
    trace: Vec<F>,
    custom_traces: Option<Vec<CustomCommitInfo<F>>>,
    air_values: Option<Vec<F>>,
    airgroup_values: Option<Vec<F>>,
}

impl<F> TraceInfo<F> {
    pub fn new(airgroup_id: usize, air_id: usize, trace: Vec<F>) -> Self {
        Self { airgroup_id, air_id, trace, custom_traces: None, air_values: None, airgroup_values: None }
    }

    pub fn with_custom_traces(mut self, custom_traces: Vec<CustomCommitInfo<F>>) -> Self {
        self.custom_traces = Some(custom_traces);
        self
    }

    pub fn with_air_values(mut self, air_values: Vec<F>) -> Self {
        self.air_values = Some(air_values);
        self
    }

    pub fn with_airgroup_values(mut self, airgroup_values: Vec<F>) -> Self {
        self.air_values = Some(airgroup_values);
        self
    }
}

pub struct FromTrace<'a, F> {
    pub trace: &'a mut dyn Trace<F>,
    pub custom_traces: Option<Vec<&'a mut dyn Trace<F>>>,
    pub air_values: Option<&'a mut dyn Values<F>>,
    pub airgroup_values: Option<&'a mut dyn Values<F>>,
}

impl<'a, F> FromTrace<'a, F> {
    pub fn new(trace: &'a mut dyn Trace<F>) -> Self {
        Self { trace, custom_traces: None, air_values: None, airgroup_values: None }
    }

    pub fn with_custom_traces(mut self, custom_traces: Vec<&'a mut dyn Trace<F>>) -> Self {
        self.custom_traces = Some(custom_traces);
        self
    }

    pub fn with_air_values(mut self, air_values: &'a mut dyn Values<F>) -> Self {
        self.air_values = Some(air_values);
        self
    }

    pub fn with_airgroup_values(mut self, airgroup_values: &'a mut dyn Values<F>) -> Self {
        self.airgroup_values = Some(airgroup_values);
        self
    }
}

/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AirInstance<F> {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub trace: Vec<F>,
    pub aux_trace: Vec<F>,
    pub custom_commits: Vec<Vec<F>>,
    pub custom_commits_extended: Vec<Vec<F>>,
    pub airgroup_values: Vec<F>,
    pub airvalues: Vec<F>,
    pub evals: Vec<F>,
    pub prover_initialized: bool,
}

impl<F: Field> AirInstance<F> {
    pub fn new(trace_info: TraceInfo<F>) -> Self {
        let airgroup_id = trace_info.airgroup_id;
        let air_id = trace_info.air_id;

        let custom_commits = Self::init_custom_commits(trace_info.custom_traces);

        let airvalues = trace_info.air_values.unwrap_or_default();

        let airgroup_values = trace_info.airgroup_values.unwrap_or_default();

        AirInstance {
            airgroup_id,
            air_id,
            trace: trace_info.trace,
            aux_trace: Vec::new(),
            custom_commits,
            custom_commits_extended: vec![Vec::new(); 10],
            airgroup_values,
            airvalues,
            evals: Vec::new(),
            prover_initialized: false,
        }
    }

    pub fn new_from_trace(mut traces: FromTrace<'_, F>) -> Self {
        let mut trace_info =
            TraceInfo::new(traces.trace.airgroup_id(), traces.trace.air_id(), traces.trace.get_buffer());

        if let Some(custom_traces) = traces.custom_traces.as_mut() {
            let mut traces = Vec::new();
            for custom_trace in custom_traces.iter_mut() {
                traces.push(CustomCommitInfo {
                    trace: custom_trace.get_buffer(),
                    commit_id: custom_trace.commit_id().unwrap(),
                });
            }
            trace_info = trace_info.with_custom_traces(traces);
        }

        if let Some(air_values) = traces.air_values.as_mut() {
            trace_info = trace_info.with_air_values(air_values.get_buffer());
        }

        AirInstance::new(trace_info)
    }

    pub fn init_custom_commits(traces_custom: Option<Vec<CustomCommitInfo<F>>>) -> Vec<Vec<F>> {
        if let Some(traces_custom) = traces_custom {
            let mut custom_commits = vec![Vec::new(); traces_custom.len()];
            for trace in traces_custom {
                custom_commits[trace.commit_id] = trace.trace;
            }
            custom_commits
        } else {
            vec![Vec::new(); 10]
        }
    }

    pub fn get_trace(&self) -> Vec<F> {
        self.trace.clone()
    }

    pub fn get_trace_stage(&self, stage: usize) -> Vec<F> {
        if stage < 2 {
            panic!("Stage must be 2 or higher");
        }

        Vec::new()
    }

    pub fn get_trace_ptr(&self) -> *mut u8 {
        self.trace.as_ptr() as *mut u8
    }

    pub fn get_evals_ptr(&self) -> *mut u8 {
        self.evals.as_ptr() as *mut u8
    }

    pub fn get_airgroup_values_ptr(&self) -> *mut u8 {
        self.airgroup_values.as_ptr() as *mut u8
    }

    pub fn get_air_values(&self) -> Vec<F> {
        self.airvalues.clone()
    }

    pub fn get_airgroup_values(&self) -> Vec<F> {
        self.airgroup_values.clone()
    }

    pub fn get_airvalues_ptr(&self) -> *mut u8 {
        self.airvalues.as_ptr() as *mut u8
    }

    pub fn init_evals(&mut self, size: usize) {
        self.evals = vec![F::zero(); size];
    }

    pub fn init_aux_trace(&mut self, size: usize) {
        self.aux_trace = create_buffer_fast(size);
    }

    pub fn init_airvalues(&mut self, size: usize) {
        self.airvalues = vec![F::zero(); size];
    }

    pub fn init_airgroup_values(&mut self, size: usize) {
        self.airgroup_values = vec![F::zero(); size];
    }

    pub fn init_custom_commit(&mut self, commit_id: usize, size: usize) {
        self.custom_commits[commit_id] = create_buffer_fast(size);
    }

    pub fn init_custom_commit_extended(&mut self, commit_id: usize, size: usize) {
        self.custom_commits_extended[commit_id] = create_buffer_fast(size);
    }

    pub fn get_aux_trace_ptr(&self) -> *mut u8 {
        match &self.aux_trace.is_empty() {
            false => self.aux_trace.as_ptr() as *mut u8,
            true => std::ptr::null_mut(), // Return null if `trace` is `None`
        }
    }

    pub fn get_custom_commits_ptr(&self) -> [*mut u8; 10] {
        let mut ptrs = [std::ptr::null_mut(); 10];
        for (i, custom_commit) in self.custom_commits.iter().enumerate() {
            ptrs[i] = custom_commit.as_ptr() as *mut u8;
        }
        ptrs
    }

    pub fn get_custom_commits_extended_ptr(&self) -> [*mut u8; 10] {
        let mut ptrs = [std::ptr::null_mut(); 10];
        for (i, custom_commit) in self.custom_commits_extended.iter().enumerate() {
            ptrs[i] = custom_commit.as_ptr() as *mut u8;
        }
        ptrs
    }

    pub fn set_prover_initialized(&mut self) {
        self.prover_initialized = true;
    }

    pub fn clear_trace(&mut self) {
        self.trace.clear();
    }

    pub fn clear_custom_commits_trace(&mut self) {
        self.custom_commits.clear();
    }
}
