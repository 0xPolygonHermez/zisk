use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use zisk_core::{opcode_execute, ZiskRequiredBinaryExtensionTable, ZiskRequiredOperation};
use zisk_pil::*;

use crate::BinaryExtensionTableSM;

pub struct BinaryExtensionSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,

    // Secondary State machines
    binary_extension_table_sm: Arc<BinaryExtensionTableSM<F>>,
}

#[derive(Debug)]
pub enum BinaryExtensionSMErr {
    InvalidOpcode,
}

impl<F: Field> BinaryExtensionSM<F> {
    const MY_NAME: &'static str = "BnaryESM";

    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        binary_extension_table_sm: Arc<BinaryExtensionTableSM<F>>,
        airgroup_id: usize,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let binary_extension_sm = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs: Mutex::new(Vec::new()),
            binary_extension_table_sm,
        };
        let binary_extension_sm = Arc::new(binary_extension_sm);

        wcm.register_component(binary_extension_sm.clone(), Some(airgroup_id), Some(air_ids));

        binary_extension_sm.binary_extension_table_sm.register_predecessor();

        binary_extension_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinaryExtensionSM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );

            self.threads_controller.wait_for_threads();

            self.binary_extension_table_sm.unregister_predecessor(scope);
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![0x0d, 0x0e, 0x0f, 0x1d, 0x1e, 0x1f, 0x23, 0x24, 0x25]
    }

    pub fn process_slice(
        input: &Vec<ZiskRequiredOperation>,
    ) -> (Vec<BinaryExtension0Row<F>>, Vec<ZiskRequiredBinaryExtensionTable>) {
        // Create the trace vector
        let mut trace = Vec::new();

        // Create the table required vector
        let mut table_required: Vec<ZiskRequiredBinaryExtensionTable> = Vec::new();

        for i in input {
            // Create an empty trace
            let mut t = BinaryExtension0Row::<F> {
                m_op: F::from_canonical_u8(i.opcode),
                ..Default::default()
            };

            // Execute the opcode
            let c: u64;
            let flag: bool;
            (c, flag) = opcode_execute(i.opcode, i.a, i.b);
            let _flag = flag;

            // Decompose the opcode into mode32 & op
            let mode32 = (i.opcode & 0x10) != 0;
            t.mode32 = F::from_bool(mode32);
            let m_op = i.opcode & 0xEF;
            t.m_op = F::from_canonical_u8(m_op);
            let mode16 = i.opcode == 0x24;
            t.mode16 = F::from_bool(mode16);
            let mode8 = i.opcode == 0x23;
            t.mode8 = F::from_bool(mode8);

            // Split a in bytes and store them in in1
            let a_bytes: [u8; 8] = i.a.to_le_bytes();
            for (i, value) in a_bytes.iter().enumerate() {
                t.in1[i] = F::from_canonical_u8(*value);
            }

            // Store b low part into in2_low
            let b_low: u64 = i.b & if mode32 { 0x1F } else { 0x3F };
            t.in2_low = F::from_canonical_u64(b_low);

            // Store b high part into free_in2
            t.free_in2[0] = F::from_canonical_u64(
                (i.b >> if mode32 { 5 } else { 6 }) & if mode32 { 0x7FF } else { 0x3FF },
            );
            t.free_in2[1] = F::from_canonical_u64((i.b >> 16) & 0xFFFF);
            t.free_in2[2] = F::from_canonical_u64((i.b >> 32) & 0xFFFF);
            t.free_in2[3] = F::from_canonical_u64(i.b >> 48);

            // Calculate out based on opcode
            match i.opcode {
                0x0d /* SLL */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position = j*8 + b_low;

                        // Calculate the 8-bits window of the result at this position
                        if position < 32 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                            t.out[j as usize][1] = F::from_canonical_u64((out >> 32) & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                            t.out[j as usize][1] = F::zero();
                        }
                    }
                },

                0x0e /* SRL */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position: i64 = j as i64*8 - b_low as i64;

                        // Calculate the 8-bits window of the result at this position
                        if position > 0 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                            t.out[j as usize][1] = F::from_canonical_u64((out >> 32) & 0xffffffff);
                        }
                        else if position > -8 {
                            let out = c & (0xff_u64 >> -position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                            t.out[j as usize][1] = F::from_canonical_u64((out >> 32) & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                            t.out[j as usize][1] = F::zero();
                        }
                    }
                },

                0x0f /* SRA */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position: i64 = j as i64*8 - b_low as i64;

                        // Calculate the 8-bits window of the result at this position
                        if position > 0 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                            t.out[j as usize][1] = F::from_canonical_u64((out >> 32) & 0xffffffff);
                        }
                        else if position > -8 {
                            let out = c & (0xff_u64 >> -position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                            t.out[j as usize][1] = F::from_canonical_u64((out >> 32) & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                            t.out[j as usize][1] = F::zero();
                        }
                    }
                },

                0x1d /* SLL_W */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position = j*8 + b_low;

                        // Calculate the 8-bits window of the result at this position
                        if position < 16 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                        }
                        t.out[j as usize][1] = F::zero();
                    }
                },

                0x1e /* SRL_W */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position: i64 = j as i64*8 - b_low as i64;

                        // Calculate the 8-bits window of the result at this position
                        if position > 0 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                        }
                        else if position > -8 {
                            let out = c & (0xff_u64 >> -position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                        }
                        t.out[j as usize][1] = F::zero();
                    }
                },

                0x1f /* SRA_W */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position: i64 = j as i64*8 - b_low as i64;

                        // Calculate the 8-bits window of the result at this position
                        if position > 0 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                        }
                        else if position > -8 {
                            let out = c & (0xff_u64 >> -position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                        }
                        t.out[j as usize][1] = F::zero();
                    }
                },

                0x23 /* SE_B */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position = j*8 + b_low;

                        // Calculate the 8-bits window of the result at this position
                        if position < 32 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                            t.out[j as usize][1] = F::from_canonical_u64((out >> 32) & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                            t.out[j as usize][1] = F::zero();
                        }
                    }
                },

                0x24 /* SE_H */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position = j*8 + b_low;

                        // Calculate the 8-bits window of the result at this position
                        if position < 32 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                            t.out[j as usize][1] = F::from_canonical_u64((out >> 32) & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                            t.out[j as usize][1] = F::zero();
                        }
                    }
                },

                0x25 /* SE_W */ => {
                    for j in 0..8 {
                        // Calculate position as the number of shifted bits for this byte
                        let position = j*8 + b_low;

                        // Calculate the 8-bits window of the result at this position
                        if position < 32 {
                            let out = c & (0xff_u64 << position);
                            t.out[j as usize][0] = F::from_canonical_u64(out & 0xffffffff);
                            t.out[j as usize][1] = F::from_canonical_u64((out >> 32) & 0xffffffff);
                        }
                        else {
                            t.out[j as usize][0] = F::zero();
                            t.out[j as usize][1] = F::zero();
                        }
                    }
                },
                _ => panic!("BinaryExtensionSM::process_slice() found invalid opcode={}", i.opcode),
            }

            // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
            t.multiplicity = F::one();

            // Store the trace in the vector
            trace.push(t);

            for a_byte in &a_bytes {
                // Create a table required
                let tr = ZiskRequiredBinaryExtensionTable {
                    opcode: m_op,
                    a: *a_byte as u64,
                    b: b_low,
                    offset: if mode32 { 5 } else { 6 },
                };

                // Store the required in the vector
                table_required.push(tr);
            }
        }

        // Return successfully
        (trace, table_required)
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryExtensionSM<F> {
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

impl<F: Field> Provable<ZiskRequiredOperation, OpResult> for BinaryExtensionSM<F> {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            let air = self
                .wcm
                .get_pctx()
                .pilout
                .get_air(BINARY_EXTENSION_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]);

            while inputs.len() >= air.num_rows() || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(air.num_rows(), inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                let binary_extension_table_sm = self.binary_extension_table_sm.clone();
                let wcm = self.wcm.clone();

                self.threads_controller.add_working_thread();
                let thread_controller = self.threads_controller.clone();

                scope.spawn(move |scope| {
                    let (trace_row, table_required) = Self::process_slice(&drained_inputs);
                    binary_extension_table_sm.prove(&table_required, false, scope);

                    info!(
                        "{}: ··· Creating Binary extension instance [{} rows]",
                        Self::MY_NAME,
                        drained_inputs.len()
                    );

                    let buffer_allocator = wcm.get_ectx().buffer_allocator.as_ref();
                    let (buffer_size, offsets) = buffer_allocator
                        .get_buffer_info(
                            wcm.get_sctx(),
                            BINARY_EXTENSION_AIRGROUP_ID,
                            BINARY_EXTENSION_AIR_IDS[0],
                        )
                        .expect("Binary extension buffer not found");

                    let trace_row_len = trace_row.len();
                    let trace_buffer = BinaryExtension0Trace::<F>::map_row_vec(trace_row, true)
                        .unwrap()
                        .buffer
                        .unwrap();
                    let mut buffer: Vec<F> = vec![F::zero(); buffer_size as usize];

                    buffer[offsets[0] as usize..
                        offsets[0] as usize + (trace_row_len * BinaryExtension0Row::<F>::ROW_SIZE)]
                        .copy_from_slice(&trace_buffer);

                    let air_instance = AirInstance::new(
                        BINARY_EXTENSION_AIRGROUP_ID,
                        BINARY_EXTENSION_AIR_IDS[0],
                        None,
                        buffer,
                    );

                    wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);

                    thread_controller.remove_working_thread();
                });
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredOperation,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());

        self.prove(&[operation], drain, scope);

        result
    }
}
