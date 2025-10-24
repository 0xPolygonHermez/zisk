use core::panic;
use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use tiny_keccak::keccakf;

use circuit::{Gate, GateOperation, PinId};
use precompiles_helpers::keccakf_topology;
use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::KeccakfFixed;
#[cfg(not(feature = "packed"))]
use zisk_pil::{KeccakfTrace, KeccakfTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{KeccakfTracePacked, KeccakfTraceRowPacked};

#[cfg(feature = "packed")]
type KeccakfTraceRowType<F> = KeccakfTraceRowPacked<F>;
#[cfg(feature = "packed")]
type KeccakfTraceType<F> = KeccakfTracePacked<F>;

#[cfg(not(feature = "packed"))]
type KeccakfTraceRowType<F> = KeccakfTraceRow<F>;
#[cfg(not(feature = "packed"))]
type KeccakfTraceType<F> = KeccakfTrace<F>;

use crate::KeccakfInput;

use super::{keccakf_constants::*, KeccakfTableGateOp, KeccakfTableSM};

use rayon::prelude::*;

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM<F: PrimeField64> {
    /// The table ID for the Keccakf Table State Machine
    table_id: usize,

    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The circuit description of the Keccakf
    program: Vec<u64>,
    gates: Vec<Gate>,

    /// Size of a circuit in the trace. It corresponds to the number of gates in the circuit.
    circuit_size: usize,

    /// Number of available circuits in the trace.
    num_available_circuits: usize,

    /// Number of available keccakfs in the trace.
    pub num_available_keccakfs: usize,

    /// Fixed columns for the Keccakf circuit.
    keccakf_fixed: KeccakfFixed<F>,
}

impl<F: PrimeField64> KeccakfSM<F> {
    /// Creates a new Keccakf State Machine instance.
    ///
    /// # Arguments
    /// * `keccakf_table_sm` - An `Arc`-wrapped reference to the Keccakf Table State Machine.
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new(sctx: Arc<SetupCtx<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        // Get the slot size
        let keccakf_top = keccakf_topology();
        let keccakf_program = keccakf_top.program;
        let keccakf_gates = keccakf_top.gates;
        let circuit_size = keccakf_program.len();

        // Compute some useful values
        let num_available_circuits = (KeccakfTraceType::<F>::NUM_ROWS - 1) / circuit_size;
        let num_available_keccakfs = NUM_KECCAKF_PER_CIRCUIT * num_available_circuits;

        // Get the fixed columns
        let airgroup_id = KeccakfTraceType::<F>::AIRGROUP_ID;
        let air_id = KeccakfTraceType::<F>::AIR_ID;
        let fixed_pols = sctx.get_fixed(airgroup_id, air_id);
        let keccakf_fixed = KeccakfFixed::new_from_vec(fixed_pols);

        // Get the table ID
        let table_id = std.get_virtual_table_id(KeccakfTableSM::TABLE_ID);

        Arc::new(Self {
            table_id,
            std,
            program: keccakf_program,
            gates: keccakf_gates,
            circuit_size,
            num_available_circuits,
            num_available_keccakfs,
            keccakf_fixed,
        })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Keccakf trace.
    /// * `num_circuits` - The number of circuits to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_trace<'a, I>(
        &self,
        trace: &mut KeccakfTraceType<F>,
        num_rows_constants: usize,
        inputs: I,
        num_inputs: usize,
    ) where
        I: IntoIterator<Item = &'a KeccakfInput>,
    {
        let mut inputs_bits: Vec<[u64; INPUT_DATA_SIZE_BITS]> =
            vec![[0u64; INPUT_DATA_SIZE_BITS]; self.num_available_circuits];

        let initial_offset = num_rows_constants;
        let input_offset = INPUT_SIZE;

        // Process the inputs
        let mut circuit = 0;
        for (i, input) in inputs.into_iter().enumerate() {
            // Get the basic data from the input
            let step_main = input.step_main;
            let addr_main = input.addr_main;
            let state = &input.state;

            circuit = i / NUM_KECCAKF_PER_CIRCUIT;
            let circuit_pos = i % NUM_KECCAKF_PER_CIRCUIT;
            let circuit_offset = circuit * self.circuit_size;

            let initial_pos = initial_offset + circuit_offset + circuit_pos;

            // Activate the in_use_clk_0 a single time
            trace[initial_pos].set_in_use_clk_0(true);

            // Fill the step_addr
            trace[initial_pos].set_step_addr(step_main);
            trace[initial_pos + STATE_SIZE].set_step_addr(addr_main as u64);

            // Activate the in_use for the input data
            for j in 0..IN_BLOCKS {
                trace[initial_pos + j * STATE_SIZE].set_in_use(true);
            }

            // Process the keccakf input
            let mut offset = initial_pos;
            state.iter().enumerate().for_each(|(j, &value)| {
                let state_offset = j * STATE_SIZE;
                let pos = offset + state_offset;

                // Process the STATE_BITS-bit chunk
                for k in 0..STATE_BITS {
                    let bit = (value >> k) & 1;

                    // Divide the value in bits:
                    //    (circuit i) [0b1011,  0b0011,  0b1000,  0b0010]
                    //    (circuit i) [1,1,0,1, 1,1,0,0, 0,0,0,1, 0,0,1,0]
                    let bit_num = k + STATE_BITS * j;
                    let old_value = inputs_bits[circuit][bit_num];
                    inputs_bits[circuit][bit_num] = (bit << circuit_pos) | old_value;

                    // We update bit[i] and val[i]
                    let bit_pos = k % MEM_BITS_IN_PARALLEL;
                    let bit_offset = (k - bit_pos) * NUM_KECCAKF_PER_CIRCUIT / MEM_BITS_IN_PARALLEL;
                    update_bit_val(
                        trace,
                        pos + bit_offset,
                        bit,
                        circuit_pos,
                        bit_pos,
                        circuit_pos == 0 && (pos + bit_offset > 1),
                    );
                }
            });

            // Apply the keccakf function and get the output
            let mut keccakf_output = *state;
            keccakf(&mut keccakf_output);

            // Activate the in_use for the output data
            offset += input_offset;
            for j in 0..OUT_BLOCKS {
                trace[offset + j * STATE_SIZE].set_in_use(true);
            }

            // Process the output
            keccakf_output.iter().enumerate().for_each(|(j, &value)| {
                let state_offset = j * STATE_SIZE;
                let pos = offset + state_offset;

                // Process the STATE_BITS-bit chunk
                for k in 0..STATE_BITS {
                    let bit = (value >> k) & 1;

                    // We update bit[i] and val[i]
                    let bit_pos = k % MEM_BITS_IN_PARALLEL;
                    let bit_offset = (k - bit_pos) * NUM_KECCAKF_PER_CIRCUIT / MEM_BITS_IN_PARALLEL;
                    update_bit_val(
                        trace,
                        pos + bit_offset,
                        bit,
                        circuit_pos,
                        bit_pos,
                        circuit_pos == 0,
                    );
                }
            });
        }

        // It the number of inputs is less than the available keccakfs, we need to fill the remaining inputs
        if num_inputs < self.num_available_keccakfs {
            // Compute the hash of zero
            let mut zero_state: [u64; 25] = [0u64; 25];
            keccakf(&mut zero_state);
            // hash_of_0: [0xf1258f7940e1dde7, 0x84d5ccf933c0478a, 0xd598261ea65aa9ee, 0xbd1547306f80494d, 0x8b284e056253d057,
            //             0xff97a42d7f8e6fd4, 0x90fee5a0a44647c4, 0x8c5bda0cd6192e76, 0xad30a6f71b19059c, 0x30935ab7d08ffc64,
            //             0xeb5aa93f2317d635, 0xa9a6e6260d712103, 0x81a57c16dbcf555f, 0x43b831cd0347c826, 0x1f22f1a11a5569f,
            //             0x5e5635a21d9ae61,  0x64befef28cc970f2, 0x613670957bc46611, 0xb87c5a554fd00ecb, 0x8c3ee88a1ccf32c8,
            //             0x940c7922ae3a2614, 0x1841f924a2c509e4, 0x16f53526e70465c2, 0x75f644e97f30a13b, 0xeaf1ff7b5ceca249]

            // If the number of inputs is not a multiple of NUM_KECCAKF_PER_CIRCUIT,
            // we fill the last processed circuit
            let rem_inputs = num_inputs % NUM_KECCAKF_PER_CIRCUIT;
            if rem_inputs != 0 {
                let circuit_offset = circuit * self.circuit_size;

                // Since no more bits are being introduced as input, we let 0 be the
                // new bits and therefore we repeat the last values
                let mut offset = initial_offset + circuit_offset;
                for j in 0..INPUT_DATA_SIZE_BITS / MEM_BITS_IN_PARALLEL {
                    let num_keccakf_offset = j * NUM_KECCAKF_PER_CIRCUIT;
                    let block = offset + num_keccakf_offset;
                    for k in rem_inputs..NUM_KECCAKF_PER_CIRCUIT {
                        let pos = block + k;
                        for l in 0..MEM_BITS_IN_PARALLEL {
                            let val = trace[pos].get_val(l);
                            trace[pos + 1].set_val(l, val);
                        }
                    }
                }

                offset += input_offset;
                // Since the new bits are all zero, we have to set the hash of 0 as the respective output
                zero_state.iter().enumerate().for_each(|(j, &value)| {
                    let state_offset = j * STATE_SIZE;
                    let state_pos = offset + state_offset;
                    for k in 0..STATE_BITS {
                        let bit = (value >> k) & 1;
                        let bit_pos = k % MEM_BITS_IN_PARALLEL;
                        let bit_offset =
                            (k - bit_pos) * NUM_KECCAKF_PER_CIRCUIT / MEM_BITS_IN_PARALLEL;
                        let pos = state_pos + bit_offset;
                        for w in rem_inputs..NUM_KECCAKF_PER_CIRCUIT {
                            update_bit_val(trace, pos + w, bit, w, bit_pos, w == 0);
                        }
                    }
                });
            }

            // Fill the remaining circuits with the hash of 0
            let next_circuit = num_inputs.div_ceil(NUM_KECCAKF_PER_CIRCUIT);
            zero_state.iter().enumerate().for_each(|(j, &value)| {
                for s in next_circuit..self.num_available_circuits {
                    let circuit_offset = s * self.circuit_size;
                    let state_offset = j * STATE_SIZE;
                    for k in 0..STATE_BITS {
                        let bit = (value >> k) & 1;
                        let bit_pos = k % MEM_BITS_IN_PARALLEL;
                        let bit_offset =
                            (k - bit_pos) * NUM_KECCAKF_PER_CIRCUIT / MEM_BITS_IN_PARALLEL;
                        let pos = initial_offset
                            + circuit_offset
                            + input_offset
                            + state_offset
                            + bit_offset;
                        for w in 0..NUM_KECCAKF_PER_CIRCUIT {
                            update_bit_val(trace, pos + w, bit, w, bit_pos, w == 0);
                        }
                    }
                }
            });
        }

        // Set the values of free_in_a, free_in_b, free_in_c
        let program = &self.program;
        let gates = &self.gates;

        let trace_rows = trace.buffer.as_mut_slice();

        let row0 = trace_rows[0];

        let mut trace_slice = &mut trace_rows[1..];
        let mut par_traces = Vec::new();

        for _ in 0..inputs_bits.len() {
            let take = self.circuit_size.min(trace_slice.len());
            let (head, tail) = trace_slice.split_at_mut(take);
            par_traces.push(head);
            trace_slice = tail;
        }

        par_traces.into_par_iter().enumerate().for_each(|(i, par_trace)| {
            for &line in program.iter() {
                let line = line as usize;
                let row = line - 1;
                let gate = &gates[line];

                // Set the value of free_in_a
                let a = &gate.pins[0];
                let ref_a = a.wired_ref as usize;
                let wired_a = a.wired_pin_id;
                let value_a;
                // If the reference is in the range of the inputs
                // and the wired pin is A (inputs are located at pin A),
                // we can get the value directly from the inputs
                if (STATE_IN_FIRST_REF
                    ..=STATE_IN_FIRST_REF
                        + (STATE_IN_NUMBER - STATE_IN_GROUP_BY) * STATE_IN_REF_DISTANCE
                            / STATE_IN_GROUP_BY
                        + (STATE_IN_GROUP_BY - 1))
                    .contains(&ref_a)
                    && ((ref_a - STATE_IN_FIRST_REF) % STATE_IN_REF_DISTANCE < STATE_IN_GROUP_BY)
                    && matches!(wired_a, PinId::A)
                {
                    let s = ref_a - STATE_IN_FIRST_REF;
                    let bit_a = (0..STATE_IN_GROUP_BY)
                        .find(|&r| (s - r) % STATE_IN_REF_DISTANCE == 0)
                        .map(|r| ((s - r) / STATE_IN_REF_DISTANCE) * STATE_IN_GROUP_BY + r)
                        .expect("Invalid bit index");
                    value_a = inputs_bits[i][bit_a];
                } else
                // Otherwise, we get one of the already computed values
                {
                    match wired_a {
                        PinId::A => {
                            value_a = if ref_a > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_a(i), ref_a - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_a(i))
                            };
                        }
                        PinId::B => {
                            value_a = if ref_a > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_b(i), ref_a - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_b(i))
                            };
                        }
                        PinId::C => {
                            value_a = if ref_a > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_c(i), ref_a - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_c(i))
                            };
                        }
                        PinId::D => {
                            value_a = if ref_a > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_d(i), ref_a - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_d(i))
                            };
                        }
                        PinId::E => panic!("Output pin E is not used by the Keccakf circuit"),
                    }
                }
                set_col(par_trace, |row, i, val| row.set_free_in_a(i, val), row, value_a);

                // Set the value of free_in_b
                let b = &gate.pins[1];
                let ref_b = b.wired_ref as usize;
                let wired_b = b.wired_pin_id;
                let value_b;
                // If the reference is in the range of the inputs
                // and the wired pin is A (inputs are located at pin A),
                // we can get the value directly from the inputs
                if (STATE_IN_FIRST_REF
                    ..=STATE_IN_FIRST_REF
                        + (STATE_IN_NUMBER - STATE_IN_GROUP_BY) * STATE_IN_REF_DISTANCE
                            / STATE_IN_GROUP_BY
                        + (STATE_IN_GROUP_BY - 1))
                    .contains(&ref_b)
                    && ((ref_b - STATE_IN_FIRST_REF) % STATE_IN_REF_DISTANCE < STATE_IN_GROUP_BY)
                    && matches!(wired_b, PinId::A)
                {
                    let s = ref_b - STATE_IN_FIRST_REF;
                    let bit_b = (0..STATE_IN_GROUP_BY)
                        .find(|&r| (s - r) % STATE_IN_REF_DISTANCE == 0)
                        .map(|r| ((s - r) / STATE_IN_REF_DISTANCE) * STATE_IN_GROUP_BY + r)
                        .expect("Invalid bit index");
                    value_b = inputs_bits[i][bit_b];
                } else
                // Otherwise, we get one of the already computed values
                {
                    match wired_b {
                        PinId::A => {
                            value_b = if ref_b > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_a(i), ref_b - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_a(i))
                            };
                        }
                        PinId::B => {
                            value_b = if ref_b > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_b(i), ref_b - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_b(i))
                            };
                        }
                        PinId::C => {
                            value_b = if ref_b > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_c(i), ref_b - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_c(i))
                            };
                        }
                        PinId::D => {
                            value_b = if ref_b > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_d(i), ref_b - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_d(i))
                            };
                        }
                        PinId::E => panic!("Output pin E is not used by the Keccakf circuit"),
                    }
                }
                set_col(par_trace, |row, i, val| row.set_free_in_b(i, val), row, value_b);

                // Set the value of free_in_c
                let c = &gate.pins[2];
                let ref_c = c.wired_ref as usize;
                let wired_c = c.wired_pin_id;
                let value_c;
                // If the reference is in the range of the inputs
                // and the wired pin is A (inputs are located at pin A),
                // we can get the value directly from the inputs
                if (STATE_IN_FIRST_REF
                    ..=STATE_IN_FIRST_REF
                        + (STATE_IN_NUMBER - STATE_IN_GROUP_BY) * STATE_IN_REF_DISTANCE
                            / STATE_IN_GROUP_BY
                        + (STATE_IN_GROUP_BY - 1))
                    .contains(&ref_c)
                    && ((ref_c - STATE_IN_FIRST_REF) % STATE_IN_REF_DISTANCE < STATE_IN_GROUP_BY)
                    && matches!(wired_c, PinId::A)
                {
                    let s = ref_c - STATE_IN_FIRST_REF;
                    let bit_c = (0..STATE_IN_GROUP_BY)
                        .find(|&r| (s - r) % STATE_IN_REF_DISTANCE == 0)
                        .map(|r| ((s - r) / STATE_IN_REF_DISTANCE) * STATE_IN_GROUP_BY + r)
                        .expect("Invalid bit index");
                    value_c = inputs_bits[i][bit_c];
                } else
                // Otherwise, we get one of the already computed values
                {
                    match wired_c {
                        PinId::A => {
                            value_c = if ref_c > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_a(i), ref_c - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_a(i))
                            };
                        }
                        PinId::B => {
                            value_c = if ref_c > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_b(i), ref_c - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_b(i))
                            };
                        }
                        PinId::C => {
                            value_c = if ref_c > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_c(i), ref_c - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_c(i))
                            };
                        }
                        PinId::D => {
                            value_c = if ref_c > 0 {
                                get_col(par_trace, |row, i| row.get_free_in_d(i), ref_c - 1)
                            } else {
                                get_col_row(&row0, |row, i| row.get_free_in_d(i))
                            };
                        }
                        PinId::E => panic!("Output pin E is not used by the Keccakf circuit"),
                    }
                }
                set_col(par_trace, |row, i, val| row.set_free_in_c(i, val), row, value_c);

                // Set the value of free_in_d
                let op = gate.op;
                let d_val = match op {
                    GateOperation::Xor => value_a ^ value_b ^ value_c,
                    GateOperation::XorAndp => {
                        value_a ^ ((value_b ^ MASK_CHUNK_BITS_KECCAKF) & value_c)
                    }
                    _ => panic!("Invalid operation"),
                };
                set_col(par_trace, |row, i, val| row.set_free_in_d(i, val), row, d_val);
            }

            // Update the multiplicity table for the circuit
            for (k, row) in par_trace.iter().enumerate().take(self.circuit_size) {
                let gate_op = self.keccakf_fixed[k + 1 + i * self.circuit_size].GATE_OP;
                let gate_op_val = match F::as_canonical_u64(&gate_op) {
                    0 => KeccakfTableGateOp::Xor,
                    1 => KeccakfTableGateOp::XorAndp,
                    _ => panic!("Invalid gate operation"),
                };

                for j in 0..CHUNKS_KECCAKF {
                    let a_val = row.get_free_in_a(j) as u64;
                    let b_val = row.get_free_in_b(j) as u64;
                    let c_val = row.get_free_in_c(j) as u64;
                    let table_row =
                        KeccakfTableSM::calculate_table_row(&gate_op_val, a_val, b_val, c_val);
                    self.std.inc_virtual_row(self.table_id, table_row as u64, 1);
                }
            }
        });

        fn update_bit_val<F: PrimeField64>(
            trace: &mut KeccakfTraceType<F>,
            pos: usize,
            bit: u64,
            circuit_pos: usize,
            bit_pos: usize,
            reset: bool,
        ) {
            trace[pos].set_bit(bit_pos, bit != 0);
            let val = if reset {
                bit << circuit_pos
            } else {
                let value = trace[pos].get_val(bit_pos);
                value + (bit << circuit_pos)
            };

            trace[pos + 1].set_val(bit_pos, val);
        }

        fn set_col<F: PrimeField64>(
            trace: &mut [KeccakfTraceRowType<F>],
            set_col_fn: impl Fn(&mut KeccakfTraceRowType<F>, usize, u8),
            index: usize,
            value: u64,
        ) {
            let mut remaining = value;
            let row = &mut trace[index];
            for i in 0..CHUNKS_KECCAKF {
                let chunk = remaining & MASK_BITS_KECCAKF;
                set_col_fn(row, i, chunk as u8);
                remaining >>= BITS_KECCAKF;
            }
        }

        fn get_col<F: PrimeField64>(
            trace: &[KeccakfTraceRowType<F>],
            get_col_fn: impl Fn(&KeccakfTraceRowType<F>, usize) -> u8,
            row_index: usize,
        ) -> u64 {
            let mut value = 0;
            let row = &trace[row_index];
            for i in 0..CHUNKS_KECCAKF {
                let col_i_val = get_col_fn(row, i) as u64;
                value += col_i_val << ((i * BITS_KECCAKF) as u64);
            }
            value
        }

        fn get_col_row<F: PrimeField64>(
            trace_row: &KeccakfTraceRowType<F>,
            get_col_fn: impl Fn(&KeccakfTraceRowType<F>, usize) -> u8,
        ) -> u64 {
            let mut value = 0;
            for i in 0..CHUNKS_KECCAKF {
                let col_i_val = get_col_fn(trace_row, i) as u64;
                value += col_i_val << ((i * BITS_KECCAKF) as u64);
            }
            value
        }
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<KeccakfInput>],
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        timer_start_trace!(KECCAKF_TRACE);
        let mut keccakf_trace = KeccakfTraceType::new_from_vec_zeroes(trace_buffer);
        let num_rows = keccakf_trace.num_rows();

        // Check that we can fit all the keccakfs in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_circuits_needed = num_inputs.div_ceil(NUM_KECCAKF_PER_CIRCUIT);
        let num_rows_constants = 1; // Number of rows used for the constants
        let num_padding_rows = (num_rows - num_rows_constants) % self.circuit_size;
        let num_rows_needed =
            num_rows_constants + num_circuits_needed * self.circuit_size + num_padding_rows;

        // Sanity checks
        debug_assert!(
            num_inputs <= self.num_available_keccakfs,
            "Exceeded available Keccakfs inputs: requested {}, but only {} are available.",
            num_inputs,
            self.num_available_keccakfs
        );
        debug_assert!(num_circuits_needed <= self.num_available_circuits);
        debug_assert!(num_rows_needed <= num_rows);

        tracing::info!(
            "··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        // Set a = 0b00..00, b = 0b11..11 and c = 0b00..00 at the first row
        // Set, e.g., the operation to be an XOR and set d = 0b11..11 = b = a ^ b ^ c
        let mut row: KeccakfTraceRowType<F> = Default::default();
        let zeros = 0u64;
        let ones = MASK_BITS_KECCAKF;
        let gate_op = self.keccakf_fixed[0].GATE_OP.as_canonical_u64();
        // Sanity check
        debug_assert_eq!(
            gate_op,
            KeccakfTableGateOp::Xor as u64,
            "Invalid initial dummy gate operation"
        );
        for i in 0..CHUNKS_KECCAKF {
            row.set_free_in_a(i, 0);
            row.set_free_in_b(i, ones as u8);
            row.set_free_in_c(i, 0);
            row.set_free_in_d(i, ones as u8);
        }
        // Update the multiplicity table
        let table_row =
            KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, zeros, ones, zeros);
        self.std.inc_virtual_row(self.table_id, table_row as u64, CHUNKS_KECCAKF as u64);

        // Assign the single constant row
        keccakf_trace[0] = row;

        // Fill the rest of the trace
        // Flatten all the inputs, since I need to process them at least in chunks of NUM_KECCAKF_PER_CIRCUIT
        let inputs = inputs.iter().flatten();
        self.process_trace(&mut keccakf_trace, num_rows_constants, inputs, num_inputs);
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);
        // A row with all zeros satisfies the constraints (since XOR(0,0,0) = 0)
        let padding_row: KeccakfTraceRowType<F> = Default::default();
        for i in (num_rows_constants + self.circuit_size * self.num_available_circuits)..num_rows {
            let gate_op = self.keccakf_fixed[i].GATE_OP.as_canonical_u64();
            // Sanity check
            debug_assert_eq!(
                gate_op,
                KeccakfTableGateOp::Xor as u64,
                "Invalid padding dummy gate operation"
            );

            let table_row =
                KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, zeros, zeros, zeros);
            self.std.inc_virtual_row(self.table_id, table_row as u64, CHUNKS_KECCAKF as u64);

            keccakf_trace[i] = padding_row;
        }
        timer_stop_and_log_trace!(KECCAKF_PADDING);

        AirInstance::new_from_trace(FromTrace::new(&mut keccakf_trace))
    }
}
