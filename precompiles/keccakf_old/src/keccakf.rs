use core::panic;
use std::sync::Arc;

use fields::PrimeField64;
use tiny_keccak::keccakf;

use circuit::{Gate, GateOperation, PinId};
use precompiles_helpers::keccakf_topology;
use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{KeccakfFixed, KeccakfTrace, KeccakfTraceRow};

use crate::KeccakfInput;

use super::{keccakf_constants::*, KeccakfTableGateOp, KeccakfTableSM};

use rayon::prelude::*;

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM {
    /// Reference to the Keccakf Table State Machine.
    keccakf_table_sm: Arc<KeccakfTableSM>,

    /// The circuit description of the Keccakf
    program: Vec<u64>,
    gates: Vec<Gate>,

    /// Size of a circuit in the trace. It corresponds to the number of gates in the circuit.
    circuit_size: usize,

    /// Number of available circuits in the trace.
    num_available_circuits: usize,

    /// Number of available keccakfs in the trace.
    pub num_available_keccakfs: usize,
}

impl KeccakfSM {
    /// Creates a new Keccakf State Machine instance.
    ///
    /// # Arguments
    /// * `keccakf_table_sm` - An `Arc`-wrapped reference to the Keccakf Table State Machine.
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new(keccakf_table_sm: Arc<KeccakfTableSM>) -> Arc<Self> {
        // Get the circuit size
        let keccakf_top = keccakf_topology();
        let keccakf_program = keccakf_top.program;
        let keccakf_gates = keccakf_top.gates;
        let circuit_size = keccakf_program.len();

        // Compute some useful values
        let num_available_circuits = (KeccakfTrace::<usize>::NUM_ROWS - 1) / circuit_size;
        let num_available_keccakfs = NUM_KECCAKF_PER_CIRCUIT * num_available_circuits;

        Arc::new(Self {
            keccakf_table_sm,
            program: keccakf_program,
            gates: keccakf_gates,
            circuit_size,
            num_available_circuits,
            num_available_keccakfs,
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
    pub fn process_trace<'a, I, F: PrimeField64>(
        &self,
        fixed: &KeccakfFixed<F>,
        trace: &mut KeccakfTrace<F>,
        num_rows_constants: usize,
        inputs: I,
        num_inputs: usize,
    ) where
        I: IntoIterator<Item = &'a KeccakfInput>,
    {
        let mut inputs_bits: Vec<[u64; INPUT_DATA_SIZE_BITS]> =
            vec![[0u64; INPUT_DATA_SIZE_BITS]; self.num_available_circuits];

        // Process the inputs
        let initial_offset = num_rows_constants;
        let input_offset = INPUT_SIZE;
        let mut circuit = 0;
        for (i, input) in inputs.into_iter().enumerate() {
            // Get the basic data from the input
            let step_received = input.step_main;
            let addr_received = input.addr_main;

            // Get the raw keccakf input
            let state_received = &input.state;

            circuit = i / NUM_KECCAKF_PER_CIRCUIT;
            let circuit_pos = i % NUM_KECCAKF_PER_CIRCUIT;
            let circuit_offset = circuit * self.circuit_size;

            // Update the multiplicity for the input
            let initial_pos = initial_offset + circuit_offset + circuit_pos;
            trace[initial_pos].in_use_clk_0 = F::ONE; // The pair (step_received, addr_received) is unique each time

            // Update the step_addr and in_use as expected
            let mut offset = initial_pos;
            for j in 0..IN_DATA_BLOCKS {
                let rb_offset = j * RB_SIZE;
                let pos = offset + rb_offset;

                trace[pos].in_use = F::ONE;

                trace[pos].step_addr = match j {
                    0 => F::from_u64(step_received), // STEP_MAIN
                    1 => F::from_u32(addr_received), // ADDR_OP
                    _ => F::ZERO,
                };
            }

            // Process the keccakf input
            state_received.iter().enumerate().for_each(|(j, &value)| {
                let block_offset = j * BLOCK_SIZE;
                let pos = offset + block_offset;

                // Process the 64-bit chunk
                for k in 0..64 {
                    let bit = (value >> k) & 1;

                    // Divide the value in bits:
                    //    (circuit i) [0b1011,  0b0011,  0b1000,  0b0010]
                    //    (circuit i) [1,1,0,1, 1,1,0,0, 0,0,0,1, 0,0,1,0]
                    let bit_num = k + 64 * j;
                    let old_value = inputs_bits[circuit][bit_num];
                    inputs_bits[circuit][bit_num] = (bit << circuit_pos) | old_value;

                    // We update bit[i] and val[i]
                    let bit_pos = k % BITS_IN_PARALLEL;
                    let bit_offset = (k - bit_pos) * NUM_KECCAKF_PER_CIRCUIT / BITS_IN_PARALLEL;
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

            // Activate the in_use for the output data
            offset += input_offset;
            for j in 0..OUT_BLOCKS {
                let rb_offset = j * RB_SIZE;
                let pos = offset + rb_offset;

                trace[pos].in_use = F::ONE;
            }

            // Apply the keccakf function and get the output
            let mut keccakf_output = *state_received;
            keccakf(&mut keccakf_output);

            // Process the output
            keccakf_output.iter().enumerate().for_each(|(j, &value)| {
                let block_offset = j * BLOCK_SIZE;
                let pos = offset + block_offset;

                // Process the 64-bit chunk
                for k in 0..64 {
                    let bit = (value >> k) & 1;

                    // We update bit[i] and val[i]
                    let bit_pos = k % BITS_IN_PARALLEL;
                    let bit_offset = (k - bit_pos) * NUM_KECCAKF_PER_CIRCUIT / BITS_IN_PARALLEL;
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
                for j in 0..INPUT_DATA_SIZE_BITS / BITS_IN_PARALLEL {
                    let block_offset = j * NUM_KECCAKF_PER_CIRCUIT;
                    let block = offset + block_offset;
                    for k in rem_inputs..NUM_KECCAKF_PER_CIRCUIT {
                        let pos = block + k;
                        for l in 0..BITS_IN_PARALLEL {
                            trace[pos + 1].val[l] = trace[pos].val[l];
                        }
                    }
                }

                offset += input_offset;
                // Since the new bits are all zero, we have to set the hash of 0 as the respective output
                zero_state.iter().enumerate().for_each(|(j, &value)| {
                    let block_offset = j * BLOCK_SIZE;
                    let block_pos = offset + block_offset;
                    for k in 0..64 {
                        let bit = (value >> k) & 1;
                        let bit_pos = k % BITS_IN_PARALLEL;
                        let bit_offset = (k - bit_pos) * NUM_KECCAKF_PER_CIRCUIT / BITS_IN_PARALLEL;
                        let pos = block_pos + bit_offset;
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
                    let block_offset = j * BLOCK_SIZE;
                    for k in 0..64 {
                        let bit = (value >> k) & 1;
                        let bit_pos = k % BITS_IN_PARALLEL;
                        let bit_offset = (k - bit_pos) * NUM_KECCAKF_PER_CIRCUIT / BITS_IN_PARALLEL;
                        let pos = initial_offset
                            + circuit_offset
                            + input_offset
                            + block_offset
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

        let row0 = trace.buffer[0];

        let mut trace_slice = &mut trace.buffer[1..];
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
                let row_a = ref_a - 1;
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
                    let mut bit_a = 0;
                    for r in 0..STATE_IN_GROUP_BY {
                        if (s - r) % STATE_IN_REF_DISTANCE == 0 {
                            let q = (s - r) / STATE_IN_REF_DISTANCE;
                            bit_a = q * STATE_IN_GROUP_BY + r;
                        }
                    }

                    value_a = inputs_bits[i][bit_a];
                } else
                // Otherwise, we get one of the already computed values
                {
                    match wired_a {
                        PinId::A => {
                            value_a = if ref_a > 0 {
                                get_col(par_trace, |row| &row.free_in_a, row_a)
                            } else {
                                get_col_row(&row0, |row| &row.free_in_a)
                            };
                        }
                        PinId::B => {
                            value_a = if ref_a > 0 {
                                get_col(par_trace, |row| &row.free_in_b, row_a)
                            } else {
                                get_col_row(&row0, |row| &row.free_in_b)
                            };
                        }
                        PinId::C => panic!("Input pin C is not used by the Keccakf circuit"),
                        PinId::D => {
                            value_a = if ref_a > 0 {
                                get_col(par_trace, |row| &row.free_in_c, row_a)
                            } else {
                                get_col_row(&row0, |row| &row.free_in_c)
                            };
                        }
                        PinId::E => panic!("Input pin E is not used by the Keccakf circuit"),
                    }
                }
                set_col(par_trace, |row| &mut row.free_in_a, row, value_a);

                // Set the value of free_in_b
                let b = &gate.pins[1];
                let ref_b = b.wired_ref as usize;
                let row_b = ref_b - 1;
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
                    let mut bit_b = 0;
                    for r in 0..STATE_IN_GROUP_BY {
                        if (s - r) % STATE_IN_REF_DISTANCE == 0 {
                            let q = (s - r) / STATE_IN_REF_DISTANCE;
                            bit_b = q * STATE_IN_GROUP_BY + r;
                        }
                    }
                    value_b = inputs_bits[i][bit_b];
                } else
                // Otherwise, we get one of the already computed values
                {
                    match wired_b {
                        PinId::A => {
                            value_b = if ref_b > 0 {
                                get_col(par_trace, |row| &row.free_in_a, row_b)
                            } else {
                                get_col_row(&row0, |row| &row.free_in_a)
                            };
                        }
                        PinId::B => {
                            value_b = if ref_b > 0 {
                                get_col(par_trace, |row| &row.free_in_b, row_b)
                            } else {
                                get_col_row(&row0, |row| &row.free_in_b)
                            };
                        }
                        PinId::C => panic!("Input pin C is not used by the Keccakf circuit"),
                        PinId::D => {
                            value_b = if ref_b > 0 {
                                get_col(par_trace, |row| &row.free_in_c, row_b)
                            } else {
                                get_col_row(&row0, |row| &row.free_in_c)
                            };
                        }
                        PinId::E => panic!("Input pin E is not used by the Keccakf circuit"),
                    }
                }
                set_col(par_trace, |row| &mut row.free_in_b, row, value_b);

                // Set the value of free_in_c as value_a OP value_b
                let op = gate.op;
                let c_val = match op {
                    GateOperation::Xor => value_a ^ value_b,
                    GateOperation::Andp => (value_a ^ MASK_CHUNK_BITS_KECCAKF) & value_b,
                    _ => panic!("Invalid operation"),
                };
                set_col(par_trace, |row| &mut row.free_in_c, row, c_val);
            }

            // Update the multiplicity table for the circuit
            for k in 0..self.circuit_size {
                let a = par_trace[k].free_in_a;
                let b = par_trace[k].free_in_b;
                let gate_op = fixed[k + 1 + i * self.circuit_size].GATE_OP;
                let gate_op_val = match F::as_canonical_u64(&gate_op) {
                    0u64 => KeccakfTableGateOp::Xor,
                    1u64 => KeccakfTableGateOp::Andp,
                    _ => panic!("Invalid gate operation"),
                };
                for j in 0..CHUNKS_KECCAKF {
                    let a_val = F::as_canonical_u64(&a[j]);
                    let b_val = F::as_canonical_u64(&b[j]);
                    let table_row = KeccakfTableSM::calculate_table_row(&gate_op_val, a_val, b_val);
                    self.keccakf_table_sm.update_input(table_row, 1);
                }
            }
        });

        fn update_bit_val<F: PrimeField64>(
            trace: &mut KeccakfTrace<F>,
            pos: usize,
            bit: u64,
            circuit_pos: usize,
            bit_pos: usize,
            reset: bool,
        ) {
            trace[pos].bit[bit_pos] = F::from_u64(bit);
            trace[pos + 1].val[bit_pos] = if reset {
                F::from_u64(bit << circuit_pos)
            } else {
                trace[pos].val[bit_pos] + F::from_u64(bit << circuit_pos)
            };
        }

        fn set_col<F: PrimeField64>(
            trace: &mut [KeccakfTraceRow<F>],
            cols: impl Fn(&mut KeccakfTraceRow<F>) -> &mut [F; CHUNKS_KECCAKF],
            index: usize,
            value: u64,
        ) {
            let mut _value = value;
            let row = &mut trace[index];
            let cols = cols(row);
            for col in cols.iter_mut() {
                *col = F::from_u64(_value & MASK_BITS_KECCAKF);
                _value >>= BITS_KECCAKF;
            }
        }

        fn get_col<F: PrimeField64>(
            trace: &[KeccakfTraceRow<F>],
            cols: impl Fn(&KeccakfTraceRow<F>) -> &[F; CHUNKS_KECCAKF],
            index: usize,
        ) -> u64 {
            let mut value = 0;
            let row = &trace[index];
            let cols = cols(row);
            for (i, col) in cols.iter().enumerate() {
                let col_i_val = F::as_canonical_u64(col);
                value += col_i_val << ((i * BITS_KECCAKF) as u64);
            }
            value
        }

        fn get_col_row<F: PrimeField64>(
            trace_row: &KeccakfTraceRow<F>,
            cols: impl Fn(&KeccakfTraceRow<F>) -> &[F; CHUNKS_KECCAKF],
        ) -> u64 {
            let mut value = 0;
            let row = trace_row;
            let cols = cols(row);
            for (i, col) in cols.iter().enumerate() {
                let col_i_val = F::as_canonical_u64(col);
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
    pub fn compute_witness<F: PrimeField64>(
        &self,
        sctx: &SetupCtx<F>,
        inputs: &[Vec<KeccakfInput>],
    ) -> AirInstance<F> {
        // Get the fixed cols
        let airgroup_id = KeccakfTrace::<usize>::AIRGROUP_ID;
        let air_id = KeccakfTrace::<usize>::AIR_ID;
        let fixed_pols = sctx.get_fixed(airgroup_id, air_id);
        let fixed = KeccakfFixed::from_vec(fixed_pols);

        timer_start_trace!(KECCAKF_TRACE);
        let mut keccakf_trace = KeccakfTrace::new_zeroes();
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

        // Set a = 0b00..00 and b = 0b11..11 at the first row
        // Set, e.g., the operation to be an XOR and set c = 0b11..11 = b = a ^ b
        let mut row: KeccakfTraceRow<F> = Default::default();
        let zeros = 0u64;
        let ones = MASK_BITS_KECCAKF;
        let gate_op = fixed[0].GATE_OP.as_canonical_u64();
        // Sanity check
        debug_assert_eq!(
            gate_op,
            KeccakfTableGateOp::Xor as u64,
            "Invalid initial dummy gate operation"
        );
        for i in 0..CHUNKS_KECCAKF {
            row.free_in_a[i] = F::from_u64(zeros);
            row.free_in_b[i] = F::from_u64(ones);
            row.free_in_c[i] = F::from_u64(ones);
        }
        // Update the multiplicity table
        let table_row = KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, zeros, ones);
        self.keccakf_table_sm.update_input(table_row, CHUNKS_KECCAKF as u64);

        // Assign the single constant row
        keccakf_trace[0] = row;

        // Fill the rest of the trace
        // Flatten all the inputs, since I need to process them at least in chunks of NUM_SHA256F_PER_CIRCUIT
        let inputs = inputs.iter().flatten();
        self.process_trace(&fixed, &mut keccakf_trace, num_rows_constants, inputs, num_inputs);
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);
        // A row with all zeros satisfies the constraints (since both XOR(0,0) and ANDP(0,0) are 0)
        let padding_row: KeccakfTraceRow<F> = Default::default();
        for i in (num_rows_constants + self.circuit_size * self.num_available_circuits)..num_rows {
            let gate_op = fixed[i].GATE_OP.as_canonical_u64();
            // Sanity check
            debug_assert_eq!(
                gate_op,
                KeccakfTableGateOp::Xor as u64,
                "Invalid padding dummy gate operation"
            );

            let table_row = KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, 0, 0);
            self.keccakf_table_sm.update_input(table_row, CHUNKS_KECCAKF as u64);

            keccakf_trace[i] = padding_row;
        }
        timer_stop_and_log_trace!(KECCAKF_PADDING);

        AirInstance::new_from_trace(FromTrace::new(&mut keccakf_trace))
    }
}
