use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{
    arith_table_inputs, ArithOperation, ArithRangeTableInputs, ArithRangeTableSM, ArithSM,
    ArithTableInputs, ArithTableSM,
};
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::Arith0Row;

const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct ArithFullSM<F> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
    arith_table_sm: Arc<ArithTableSM<F>>,
    arith_range_table_sm: Arc<ArithRangeTableSM<F>>,
}

impl<F: Field> ArithFullSM<F> {
    const MY_NAME: &'static str = "Arith   ";
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        arith_table_sm: Arc<ArithTableSM<F>>,
        arith_range_table_sm: Arc<ArithRangeTableSM<F>>,
        airgroup_id: usize,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let arith_full_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs: Mutex::new(Vec::new()),
            arith_table_sm,
            arith_range_table_sm,
        };
        let arith_full_sm = Arc::new(arith_full_sm);

        wcm.register_component(arith_full_sm.clone(), Some(airgroup_id), Some(air_ids));

        arith_full_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <ArithFullSM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
            self.threads_controller.wait_for_threads();

            self.arith_table_sm.unregister_predecessor(scope);
            self.arith_range_table_sm.unregister_predecessor(scope);
        }
    }
    pub fn process_slice(
        input: &Vec<ZiskRequiredOperation>,
        range_table_inputs: &mut ArithRangeTableInputs<F>,
        table_inputs: &mut ArithTableInputs<F>,
    ) -> Vec<Arith0Row<F>> {
        let mut traces: Vec<Arith0Row<F>> = Vec::new();
        let mut aop = ArithOperation::new();
        for input in input.iter() {
            aop.calculate(input.opcode, input.a, input.b);
            let mut t: Arith0Row<F> = Default::default();
            for i in 0..4 {
                t.a[i] = F::from_canonical_u64(aop.a[i]);
                t.b[i] = F::from_canonical_u64(aop.b[i]);
                t.c[i] = F::from_canonical_u64(aop.c[i]);
                t.d[i] = F::from_canonical_u64(aop.d[i]);
                // arith_operation.a[i];
            }
            // range_table_inputs.push(0, 0);
            // table_inputs.fast_push(0, 0, 0);
            t.m32 = F::from_bool(aop.m32);
            t.div = F::from_bool(aop.div);
            t.na = F::from_bool(aop.na);
            t.nb = F::from_bool(aop.nb);
            t.np = F::from_bool(aop.np);
            t.nr = F::from_bool(aop.nr);
            t.signed = F::from_bool(aop.signed);
            t.main_mul = F::from_bool(aop.main_mul);
            t.main_div = F::from_bool(aop.main_div);
            t.sext = F::from_bool(aop.sext);
            t.multiplicity = F::one();

            t.fab = if aop.na != aop.nb { F::neg_one() } else { F::one() };
            //  na * (1 - 2 * nb);
            t.na_fb = if aop.na {
                if aop.nb {
                    F::neg_one()
                } else {
                    F::one()
                }
            } else {
                F::zero()
            };
            t.nb_fa = if aop.nb {
                if aop.na {
                    F::neg_one()
                } else {
                    F::one()
                }
            } else {
                F::zero()
            };
            t.bus_res1 = F::from_canonical_u64(
                if aop.sext { 0xFFFFFFFF } else { 0 }
                    + if aop.main_mul {
                        aop.c[2] + aop.c[3] << 16
                    } else if aop.main_div {
                        aop.a[2] + aop.a[3] << 16
                    } else {
                        aop.d[2] + aop.d[3] << 16
                    },
            );

            traces.push(t);
        }
        // range_table_inputs.push(0, 0);
        //table_inputs.fast_push(0, 0, 0);
        traces
    }
}

impl<F: Field> WitnessComponent<F> for ArithFullSM<F> {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}

impl<F: Field> Provable<ZiskRequiredOperation, OpResult> for ArithFullSM<F> {
    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                if drain && !inputs.is_empty() {
                    println!("ArithFullSM: Draining inputs");
                }

                // self.threads_controller.add_working_thread();
                // let thread_controller = self.threads_controller.clone();

                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let _drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                scope.spawn(move |_| {
                    let mut arith_range_table_inputs = ArithRangeTableInputs::<F>::new();
                    let mut arith_table_inputs = ArithTableInputs::<F>::new();
                    let _trace = Self::process_slice(
                        &_drained_inputs,
                        &mut arith_range_table_inputs,
                        &mut arith_table_inputs,
                    );
                    // thread_controller.remove_working_thread();
                    // TODO! Implement prove drained_inputs (a chunk of operations)
                });
            }
        }
    }
}
