use log::error;
use std::mem;

use proofman::executor::Executor;
use core::trace::{Trace, StoreType};
use core::trace::trace_layout::TraceLayout;

use std::sync::Mutex;
use std::sync::{Arc, RwLock};

use proofman::proof_ctx::ProofCtx;

use math::FieldElement;
use math::fields::f64::BaseElement;

pub struct Fibonacci {
    name: String
}

impl Fibonacci {
    pub fn new() -> Self {
        Fibonacci { name: "Fibonacci ".to_string() }
    }

    fn get_fibonacci_trace(&self, num_rows: usize) -> Trace {
        let mut trace_layout = TraceLayout::new(num_rows);

        trace_layout.add_column("witness.a".to_string(), mem::size_of::<BaseElement>());
        trace_layout.add_column("witness.b".to_string(), mem::size_of::<BaseElement>());
        trace_layout.add_column("fixed.L1".to_string(), mem::size_of::<BaseElement>());
        trace_layout.add_column("fixed.LLAST".to_string(), mem::size_of::<BaseElement>());

        // Create the Trace
        let mut trace = Trace::new(&trace_layout, StoreType::RowMajor);
        trace.new_trace(num_rows);

        let mut witness_a = vec![BaseElement::default(); num_rows];
        let mut witness_b = vec![BaseElement::default(); num_rows];
        let mut fixed_l1 = vec![BaseElement::default(); num_rows];
        let mut fixed_llast = vec![BaseElement::default(); num_rows];

        witness_a[0] = BaseElement::new(1);
        witness_b[0] = BaseElement::new(1);
        for i in 1..num_rows {
            let temp = witness_a[i - 1];
            witness_a[i] = witness_b[i - 1];
            witness_b[i] = temp + witness_b[i - 1];
        }
        fixed_l1[0] = BaseElement::new(1);
        fixed_llast[num_rows - 1] = BaseElement::new(1);

        trace.set_column_u8("witness.a", witness_a.len(), FieldElement::elements_as_bytes(&witness_a));
        trace.set_column_u8("witness.b", witness_b.len(), FieldElement::elements_as_bytes(&witness_b));
        trace.set_column_u8("fixed.L1", fixed_l1.len(), FieldElement::elements_as_bytes(&fixed_l1));
        trace.set_column_u8("fixed.LLAST", fixed_llast.len(), FieldElement::elements_as_bytes(&fixed_llast));

        trace
    }
}

impl Executor for Fibonacci {
    fn witness_computation(&self, stage_id: u32, subproof_id: u32, instance_id: i32, proof_ctx: Arc<RwLock<ProofCtx>>/*, publics*/) {
        if stage_id != 1 { return; }
        
        if instance_id != -1 {
            error!("[{}] Air instance id already existing in stageId 1.", self.name);
            panic!("[{}] Air instance id already existing in stageId 1.", self.name);
        }

        let mut proof_ctx = proof_ctx.write().unwrap();
        
        // Create the Trace Layout and store it in the Proof Context
        let trace = self.get_fibonacci_trace(2usize.pow(4));
        proof_ctx.add_air_instance(subproof_id as usize, 0, Arc::new(Mutex::new(trace)));
    }
}
