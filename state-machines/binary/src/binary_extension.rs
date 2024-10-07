use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::BinaryExtensionTableSM;
use log::info;
use num_bigint::BigInt;
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use proofman_util::create_buffer_fast;
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use zisk_core::{ZiskRequiredBinaryExtensionTable, ZiskRequiredOperation, ZiskRequiredRangeCheck};
use zisk_pil::*;

const MASK_32: u64 = 0xFFFFFFFF;
const MASK_64: u64 = 0xFFFFFFFFFFFFFFFF;

const SE_MASK_32: u64 = 0xFFFFFFFF00000000;
const SE_MASK_16: u64 = 0xFFFFFFFFFFFF0000;
const SE_MASK_8: u64 = 0xFFFFFFFFFFFFFF00;

const SIGN_32_BIT: u64 = 0x80000000;
const SIGN_BYTE: u64 = 0x80;

const LS_5_BITS: u64 = 0x1F;
const LS_6_BITS: u64 = 0x3F;

pub struct BinaryExtensionSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // STD
    std: Arc<Std<F>>,

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

impl<F: PrimeField> BinaryExtensionSM<F> {
    const MY_NAME: &'static str = "BinaryE ";

    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        std: Arc<Std<F>>,
        binary_extension_table_sm: Arc<BinaryExtensionTableSM<F>>,
        airgroup_id: usize,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let binary_extension_sm = Self {
            wcm: wcm.clone(),
            std: std.clone(),
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs: Mutex::new(Vec::new()),
            binary_extension_table_sm,
        };
        let binary_extension_sm = Arc::new(binary_extension_sm);

        wcm.register_component(binary_extension_sm.clone(), Some(airgroup_id), Some(air_ids));

        std.register_predecessor();

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

            self.std.unregister_predecessor(self.wcm.get_arc_pctx(), None);
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![0x0d, 0x0e, 0x0f, 0x1d, 0x1e, 0x1f, 0x23, 0x24, 0x25]
    }

    pub fn process_slice(
        input: &Vec<ZiskRequiredOperation>,
    ) -> (
        Vec<BinaryExtension0Row<F>>,
        Vec<ZiskRequiredBinaryExtensionTable>,
        Vec<ZiskRequiredRangeCheck>,
    ) {
        // Create the trace vector
        let mut trace = Vec::with_capacity(input.len());

        // Create the table required vector
        let mut table_required: Vec<ZiskRequiredBinaryExtensionTable> =
            Vec::with_capacity(input.len() * 8);

        // Create the range check vector
        let mut range_check: Vec<ZiskRequiredRangeCheck> = Vec::new();

        for i in input {
            // Get the opcode
            let op = i.opcode;

            // Create an empty trace
            let mut t =
                BinaryExtension0Row::<F> { op: F::from_canonical_u8(op), ..Default::default() };

            // Set if the opcode is a shift operation
            let op_is_shift = (op == 0x0d) ||
                (op == 0x0e) ||
                (op == 0x0f) ||
                (op == 0x1d) ||
                (op == 0x1e) ||
                (op == 0x1f);
            t.op_is_shift = F::from_bool(op_is_shift);

            // Set if the opcode is a shift word operation
            let op_is_shift_word = (op == 0x1d) || (op == 0x1e) || (op == 0x1f);

            // Detect if this is a sign extend operation
            let a = if op_is_shift { i.a } else { i.b };
            let b = if op_is_shift { i.b } else { i.a };

            // Split a in bytes and store them in in1
            let a_bytes: [u8; 8] = a.to_le_bytes();
            for (i, value) in a_bytes.iter().enumerate() {
                t.in1[i] = F::from_canonical_u8(*value);
            }

            // Store b low part into in2_low
            let in2_low: u64 = if op_is_shift { b & 0xFF } else { 0 };
            t.in2_low = F::from_canonical_u64(in2_low);

            // Store b lower bits when shifting, depending on operation size
            let b_low = if op_is_shift_word { b & LS_5_BITS } else { b & LS_6_BITS };

            // Store b into in2
            let in2_0: u64 = if op_is_shift { (b >> 8) & 0xFFFFFF } else { b & 0xFFFFFFFF };
            let in2_1: u64 = (b >> 32) & 0xFFFFFFFF;
            t.in2[0] = F::from_canonical_u64(in2_0);
            t.in2[1] = F::from_canonical_u64(in2_1);

            // Set main SM step
            t.main_step = F::from_canonical_u64(i.step);

            // Calculate the trace output
            let mut t_out: [[u64; 2]; 8] = [[0; 2]; 8];

            // Calculate output based on opcode
            match i.opcode {
                0x0d /* SLL */ => {
                    for j in 0..8 {
                        let bits_to_shift = b_low + 8*j as u64;
                        let out = if bits_to_shift < 64 { (a_bytes[j] as u64) << bits_to_shift } else { 0 };
                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },

                0x0e /* SRL */ => {
                    for j in 0..8 {
                        let out = ((a_bytes[j] as u64) << (8*j as u64)) >> b_low;
                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },

                0x0f /* SRA */ => {
                    for j in 0..8 {
                        let mut out = ((a_bytes[j] as u64) << (8*j as u64)) >> b_low;
                        if j == 7 {
                            // most significant bit of most significant byte define if negative or not
                            // if negative then add b bits one on the left
                            if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                                out |= MASK_64 << (64 - b_low);
                            }
                        }
                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },

                0x1d /* SLL_W */ => {
                    for j in 0..8 {
                        let mut out: u64;
                        if j >= 4 {
                            out = 0;
                        }
                        else {
                            out = (((a_bytes[j] as u64) << b_low) + (8 * j as u64)) & MASK_32;
                            if (out & SIGN_32_BIT) != 0 {
                                out |= SE_MASK_32;
                            }
                        }
                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },

                0x1e /* SRL_W */ => {
                    for j in 0..8 {
                        let mut out: u64;
                        if j >= 4 {
                            out = 0;
                        } else {
                            out = (((a_bytes[j] as u64) << (8 * j as u64)) >> b_low) & MASK_32;
                            if (out & SIGN_32_BIT) != 0 {
                                out |= SE_MASK_32;
                            }
                        }
                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },

                0x1f /* SRA_W */ => {
                    for j in 0..8 {
                        let mut out: u64;
                        if j >= 4 {
                            out = 0;
                        } else {
                            out = ((a_bytes[j] as u64) << (8 * j as u64)) >> b_low;
                            if j == 3 && ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                                    out |= MASK_64 << (32 - b_low);
                            }
                        }
                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },

                0x23 /* SE_B */ => {
                    for j in 0..8 {
                        let out: u64;
                        if j == 0 {
                            if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                                out = (a_bytes[j] as u64) | SE_MASK_8;
                            } else {
                                out = a_bytes[j] as u64;
                            }
                        } else {
                            out = 0;
                        }
                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },

                0x24 /* SE_H */ => {
                    for j in 0..8 {
                        let out: u64;
                        if j == 0 {
                            out = a_bytes[j] as u64;
                        } else if j == 1 {
                            if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                                out = (a_bytes[j] as u64) | SE_MASK_16;
                            } else {
                                out = a_bytes[j] as u64;
                            }
                        } else {
                            out = 0;
                        }
                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },

                0x25 /* SE_W */ => {
                    for j in 0..4 {
                        let mut out = (a_bytes[j] as u64) << (8 * j as u64);
                        if j == 3 &&
                             ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                                out |= SE_MASK_32;
                            }

                        t_out[j][0] = out & 0xffffffff;
                        t_out[j][1] = (out >> 32) & 0xffffffff;
                    }
                },
                _ => panic!("BinaryExtensionSM::process_slice() found invalid opcode={}", i.opcode),
            }

            // Convert the trace output to field elements
            for j in 0..8 {
                t.out[j as usize][0] = F::from_canonical_u64(t_out[j as usize][0]);
                t.out[j as usize][1] = F::from_canonical_u64(t_out[j as usize][1]);
            }

            // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
            t.multiplicity = F::one();

            // Store the trace in the vector
            trace.push(t);

            for (i, a_byte) in a_bytes.iter().enumerate() {
                // Create a table required
                let tr = ZiskRequiredBinaryExtensionTable {
                    opcode: op,
                    a,
                    b,
                    offset: i as u64,
                    row: BinaryExtensionTableSM::<F>::calculate_table_row(
                        op,
                        i as u64,
                        *a_byte as u64,
                        in2_low,
                    ),
                    multiplicity: 1,
                };

                // Store the required in the vector
                table_required.push(tr);
            }

            // Store the range check
            if op_is_shift {
                let rc = ZiskRequiredRangeCheck { rc: in2_0 };
                range_check.push(rc);
            }
        }

        // Return successfully
        (trace, table_required, range_check)
    }
}

impl<F: PrimeField> WitnessComponent<F> for BinaryExtensionSM<F> {}

impl<F: PrimeField> Provable<ZiskRequiredOperation, OpResult> for BinaryExtensionSM<F> {
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

                let std_cloned = self.std.clone();

                scope.spawn(move |scope| {
                    let (mut trace_row, mut table_required, range_check) =
                        Self::process_slice(&drained_inputs);

                    let air = wcm
                        .get_pctx()
                        .pilout
                        .get_air(BINARY_EXTENSION_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]);

                    info!(
                        "{}: ··· Creating Binary extension instance [{} / {} rows filled {:.2}%]",
                        Self::MY_NAME,
                        drained_inputs.len(),
                        air.num_rows(),
                        drained_inputs.len() as f64 / air.num_rows() as f64 * 100.0
                    );

                    let mut trace_row_len = trace_row.len();

                    if drain && (air.num_rows() > trace_row_len) {
                        let padding_size = air.num_rows() - trace_row_len;

                        let padding_row = BinaryExtension0Row::<F> {
                            op: F::from_canonical_u64(0x25),
                            ..Default::default()
                        };

                        for _ in trace_row_len..air.num_rows() {
                            trace_row.push(padding_row);
                        }

                        for i in 0..8 {
                            table_required.push(ZiskRequiredBinaryExtensionTable {
                                opcode: 0,
                                a: 0,
                                b: 0,
                                offset: i,
                                row: BinaryExtensionTableSM::<F>::calculate_table_row(
                                    0x25, i, 0, 0,
                                ),
                                multiplicity: padding_size as u64,
                            });
                        }

                        trace_row_len = trace_row.len();
                    }

                    binary_extension_table_sm.prove(&table_required, false, scope);

                    for rc in range_check {
                        std_cloned.range_check(
                            F::from_canonical_u64(rc.rc),
                            BigInt::from(0),
                            BigInt::from(0xFFFFFF),
                        );
                    }

                    let buffer_allocator = wcm.get_ectx().buffer_allocator.as_ref();
                    let (buffer_size, offsets) = buffer_allocator
                        .get_buffer_info(
                            wcm.get_sctx(),
                            BINARY_EXTENSION_AIRGROUP_ID,
                            BINARY_EXTENSION_AIR_IDS[0],
                        )
                        .expect("Binary extension buffer not found");

                    let trace_buffer = BinaryExtension0Trace::<F>::map_row_vec(trace_row, true)
                        .unwrap()
                        .buffer
                        .unwrap();

                    let mut buffer = create_buffer_fast(buffer_size as usize);
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
}
