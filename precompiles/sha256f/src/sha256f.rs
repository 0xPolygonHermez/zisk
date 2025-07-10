use core::panic;
use std::{fs, path::PathBuf, sync::Arc};

use fields::PrimeField64;
use generic_array::{typenum::U64, GenericArray};
use sha2::compress256;

use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_core::{convert_u64_to_generic_array_bytes, convert_u64_to_u32};
use zisk_pil::{Sha256fFixed, Sha256fTrace, Sha256fTraceRow};

use crate::Sha256fInput;

use super::{sha256f_constants::*, InputType, Script, Sha256fTableGateOp, Sha256fTableSM};

use rayon::prelude::*;

/// The `Sha256fSM` struct encapsulates the logic of the Sha256f State Machine.
pub struct Sha256fSM<F: PrimeField64> {
    /// Reference to the Sha256f Table State Machine.
    sha256f_table_sm: Arc<Sha256fTableSM>,

    /// Script for the Sha256f's circuit representation
    script: Arc<Script>,

    /// Size of a circuit in the trace. It corresponds to the number of gates in the circuit.
    pub circuit_size: usize,

    /// Number of available circuits in the trace.
    num_available_circuits: usize,

    /// Number of available sha256fs in the trace.
    pub num_available_sha256fs: usize,

    sha256f_fixed: Sha256fFixed<F>,
}

impl<F: PrimeField64> Sha256fSM<F> {
    /// Creates a new Sha256f State Machine instance.
    ///
    /// # Arguments
    /// * `sha256f_table_sm` - An `Arc`-wrapped reference to the Sha256f Table State Machine.
    ///
    /// # Returns
    /// A new `Sha256fSM` instance.
    pub fn new(
        sctx: Arc<SetupCtx<F>>,
        sha256f_table_sm: Arc<Sha256fTableSM>,
        script_path: PathBuf,
    ) -> Arc<Self> {
        let script = fs::read_to_string(script_path).expect("Failed to read sha256f_script.json");
        let script: Script =
            serde_json::from_str(&script).expect("Failed to parse sha256f_script.json");

        // Get the circuit size
        let circuit_size = script.total;
        let circuit_ops_count = &script.sums;

        // Check that the script is valid
        assert!(
            circuit_ops_count.xor
                + circuit_ops_count.ch
                + circuit_ops_count.maj
                + circuit_ops_count.add
                == circuit_size + 1
        );
        assert!(script.program.len() == circuit_size);

        // Compute some useful values
        let num_available_circuits = (Sha256fTrace::<usize>::NUM_ROWS - 1) / circuit_size;
        let num_available_sha256fs = NUM_SHA256F_PER_CIRCUIT * num_available_circuits;

        let airgroup_id = Sha256fTrace::<usize>::AIRGROUP_ID;
        let air_id = Sha256fTrace::<usize>::AIR_ID;
        let fixed_pols = sctx.get_fixed(airgroup_id, air_id);
        let sha256f_fixed = Sha256fFixed::from_vec(fixed_pols);

        Arc::new(Self {
            sha256f_table_sm,
            script: Arc::new(script),
            circuit_size,
            num_available_circuits,
            num_available_sha256fs,
            sha256f_fixed,
        })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Sha256f trace.
    /// * `num_circuits` - The number of circuits to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_trace<'a, I>(
        &self,
        trace: &mut Sha256fTrace<F>,
        num_rows_constants: usize,
        inputs: I,
        num_inputs: usize,
    ) where
        I: IntoIterator<Item = &'a Sha256fInput>,
    {
        let mut inputs_bits: Vec<[u64; INPUT_DATA_SIZE_BITS]> =
            vec![[0u64; INPUT_DATA_SIZE_BITS]; self.num_available_circuits];

        // Process the inputs
        let initial_offset = num_rows_constants;
        let input_offset = INPUT_SIZE;
        let output_offset = OUTPUT_SIZE;
        let mut circuit = 0;
        for (i, input) in inputs.into_iter().enumerate() {
            // Get the basic data from the input
            let step_received = input.step_main;
            let addr_received = input.addr_main;
            let state_addr_received = input.state_addr;
            let input_addr_received = input.input_addr;
            let state_received = &input.state;
            let input_received = &input.input;

            // Convert the input data to 32-bit words
            let mut state_u32: [u32; 8] = convert_u64_to_u32(state_received).try_into().unwrap();
            let input_u32: [u32; 16] = convert_u64_to_u32(input_received).try_into().unwrap();

            // And collect them
            let mut sha256f_input_data = [0u32; INPUT_DATA_SIZE_U32];
            sha256f_input_data[..8].copy_from_slice(&state_u32);
            sha256f_input_data[8..].copy_from_slice(&input_u32);

            circuit = i / NUM_SHA256F_PER_CIRCUIT;
            let circuit_pos = i % NUM_SHA256F_PER_CIRCUIT;
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
                    0 => F::from_u64(step_received),       // STEP_MAIN
                    1 => F::from_u32(addr_received),       // ADDR_OP
                    2 => F::from_u32(state_addr_received), // ADDR_STATE
                    3 => F::from_u32(input_addr_received), // ADDR_INPUT
                    4 => F::from_u32(state_addr_received), // ADDR_IND_0
                    5 => F::from_u32(input_addr_received), // ADDR_IND_1
                    _ => F::ZERO,
                };
            }

            // Process the input data
            sha256f_input_data.iter().enumerate().for_each(|(j, &value)| {
                let block_offset = j * BLOCK_SIZE;
                let pos = offset + block_offset;

                // Process the 32-bit chunk
                for k in 0..32 {
                    let bit = ((value >> k) & 1) as u64;

                    // Divide the value in bits:
                    //    (circuit i) [0b1011,  0b0011,  0b1000,  0b0010]
                    //    (circuit i) [1,0,1,1, 0,0,1,1, 1,0,0,0, 0,0,1,0]
                    let bit_pos = k + 32 * j;
                    let old_value = inputs_bits[circuit][bit_pos];
                    inputs_bits[circuit][bit_pos] = (bit << circuit_pos) | old_value;

                    // We update bit[i] and val[i]
                    let bit_pos = k % BITS_IN_PARALLEL;
                    let bit_offset = (k - bit_pos) * NUM_SHA256F_PER_CIRCUIT / BITS_IN_PARALLEL;
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

            // Obtain the sha256f output
            let block: GenericArray<u8, U64> = convert_u64_to_generic_array_bytes(input_received);
            compress256(&mut state_u32, &[block]);

            // Process the output
            state_u32.iter().enumerate().for_each(|(j, &value)| {
                let block_offset = j * BLOCK_SIZE;
                let pos = offset + block_offset;

                // Process the 32-bit chunk
                for k in 0..32 {
                    let bit = ((value >> k) & 1) as u64;

                    // We update bit[i] and val[i]
                    let bit_pos = k % BITS_IN_PARALLEL;
                    let bit_offset = (k - bit_pos) * NUM_SHA256F_PER_CIRCUIT / BITS_IN_PARALLEL;
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

            // Finally, activate the in_use for the two indirections
            offset += output_offset;
            for j in 0..2 {
                let rb_offset = j * RB_SIZE;
                let pos = offset + rb_offset;

                trace[pos].in_use = F::ONE;
            }
        }

        // It the number of inputs is less than the available sha256fs, we need to fill the remaining inputs
        if num_inputs < self.num_available_sha256fs {
            // Compute the hash of zero
            let mut zero_state = [0u32; 8];
            let block_zeros: GenericArray<u8, U64> = GenericArray::default();
            compress256(&mut zero_state, &[block_zeros]);
            // hash_of_0: [0x7ca51614, 0x425c3ba8, 0xce54dd2f, 0xc2020ae7, 0xb6e574d1, 0x98136d0f, 0xae7e26cc, 0xbf0be7a6]

            // If the number of inputs is not a multiple of NUM_SHA256F_PER_CIRCUIT,
            // we fill the last processed circuit
            let rem_inputs = num_inputs % NUM_SHA256F_PER_CIRCUIT;
            if rem_inputs != 0 {
                let circuit_offset = circuit * self.circuit_size;

                // Since no more bits are being introduced as input, we let 0 be the
                // new bits and therefore we repeat the last values
                let mut offset = initial_offset + circuit_offset;
                for j in 0..INPUT_DATA_SIZE_BITS / BITS_IN_PARALLEL {
                    let block_offset = j * NUM_SHA256F_PER_CIRCUIT;
                    let block = offset + block_offset;
                    for k in rem_inputs..NUM_SHA256F_PER_CIRCUIT {
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
                    for k in 0..32 {
                        let bit = ((value >> k) & 1) as u64;
                        let bit_pos = k % BITS_IN_PARALLEL;
                        let bit_offset = (k - bit_pos) * NUM_SHA256F_PER_CIRCUIT / BITS_IN_PARALLEL;
                        let pos = block_pos + bit_offset;
                        for w in rem_inputs..NUM_SHA256F_PER_CIRCUIT {
                            update_bit_val(trace, pos + w, bit, w, bit_pos, w == 0);
                        }
                    }
                });
            }

            // Fill the remaining circuits with the hash of 0
            let next_circuit = num_inputs.div_ceil(NUM_SHA256F_PER_CIRCUIT);
            zero_state.iter().enumerate().for_each(|(j, &value)| {
                for s in next_circuit..self.num_available_circuits {
                    let circuit_offset = s * self.circuit_size;
                    let block_offset = j * BLOCK_SIZE;
                    for k in 0..32 {
                        let bit = ((value >> k) & 1) as u64;
                        let bit_pos = k % BITS_IN_PARALLEL;
                        let bit_offset = (k - bit_pos) * NUM_SHA256F_PER_CIRCUIT / BITS_IN_PARALLEL;
                        let pos = initial_offset
                            + circuit_offset
                            + input_offset
                            + block_offset
                            + bit_offset;
                        for w in 0..NUM_SHA256F_PER_CIRCUIT {
                            update_bit_val(trace, pos + w, bit, w, bit_pos, w == 0);
                        }
                    }
                }
            });
        }

        // 2] Set the values of free_in_a, free_in_b, free_in_c and free_in_d using the script

        // Divide input bits between state bits and hash input bits
        let state_bits: Vec<[u64; STATE_SIZE_BITS]> =
            inputs_bits.iter().map(|bits| bits[..STATE_SIZE_BITS].try_into().unwrap()).collect();
        let hash_input_bits: Vec<[u64; INPUT_SIZE_BITS]> =
            inputs_bits.iter().map(|bits| bits[STATE_SIZE_BITS..].try_into().unwrap()).collect();

        let trace_rows = trace.row_slice_mut();

        let row0 = trace_rows[0];

        let mut trace_slice = &mut trace_rows[1..];

        let mut par_traces = Vec::new();

        for _ in 0..self.num_available_circuits {
            let take = self.circuit_size.min(trace_slice.len());
            let (head, tail) = trace_slice.split_at_mut(take);
            par_traces.push(head);
            trace_slice = tail;
        }

        let program = &self.script.program;
        par_traces.into_par_iter().enumerate().for_each(|(i, par_trace)| {
            for line in program.iter() {
                let row = line.ref_ - 1;

                let a_val = get_val(par_trace, &row0, &state_bits, &hash_input_bits, i, &line.in1);
                set_col(par_trace, |row| &mut row.free_in_a, row, a_val);

                let b_val = get_val(par_trace, &row0, &state_bits, &hash_input_bits, i, &line.in2);
                set_col(par_trace, |row| &mut row.free_in_b, row, b_val);

                if let Some(in3) = &line.in3 {
                    let c_val = get_val(par_trace, &row0, &state_bits, &hash_input_bits, i, in3);
                    set_col(par_trace, |row| &mut row.free_in_c, row, c_val);
                }
                let c_val = get_col(par_trace, |row| &row.free_in_c, row);

                let op = &line.op;
                let d_val;
                let op_val;
                if op == "xor" {
                    d_val = a_val ^ b_val ^ c_val;
                    op_val = Sha256fTableGateOp::Xor;
                } else if op == "ch" {
                    d_val = (a_val & b_val) ^ ((a_val ^ MASK_CHUNK_BITS_SHA256F) & c_val);
                    op_val = Sha256fTableGateOp::Ch;
                } else if op == "maj" {
                    d_val = (a_val & b_val) ^ (a_val & c_val) ^ (b_val & c_val);
                    op_val = Sha256fTableGateOp::Maj;
                } else if op == "add" {
                    d_val = a_val ^ b_val ^ c_val;
                    op_val = Sha256fTableGateOp::Add;

                    // Compute and set the carry
                    let carry = (a_val & b_val) | (a_val & c_val) | (b_val & c_val);
                    set_col(par_trace, |row| &mut row.carry, row, carry);
                    set_col(par_trace, |row| &mut row.free_in_c, row + 1, carry);
                } else {
                    panic!("Invalid operation: {}", op);
                }

                set_col(par_trace, |row| &mut row.free_in_d, row, d_val);

                // Update the multiplicity table for the circuit
                for j in 0..CHUNKS_SHA256F {
                    let a = (a_val >> (j * BITS_SHA256F)) & MASK_BITS_SHA256F;
                    let b = (b_val >> (j * BITS_SHA256F)) & MASK_BITS_SHA256F;
                    let c = (c_val >> (j * BITS_SHA256F)) & MASK_BITS_SHA256F;
                    let table_row = Sha256fTableSM::calculate_table_row(&op_val, a, b, c);
                    self.sha256f_table_sm.update_input(table_row, 1);
                }
            }
        });

        fn update_bit_val<F: PrimeField64>(
            trace: &mut Sha256fTrace<F>,
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

        fn get_val<F: PrimeField64>(
            trace: &[Sha256fTraceRow<F>],
            row0: &Sha256fTraceRow<F>,
            state_bits: &[[u64; STATE_SIZE_BITS]],
            hash_input_bits: &[[u64; INPUT_SIZE_BITS]],
            circuit: usize,
            gate_input: &InputType,
        ) -> u64 {
            match gate_input {
                InputType::Wired { gate, pin, .. } => match pin.as_str() {
                    "in1" => {
                        if *gate > 0 {
                            get_col(trace, |row| &row.free_in_a, *gate - 1)
                        } else {
                            get_col_row(row0, |row| &row.free_in_a)
                        }
                    }
                    "in2" => {
                        if *gate > 0 {
                            get_col(trace, |row| &row.free_in_b, *gate - 1)
                        } else {
                            get_col_row(row0, |row| &row.free_in_b)
                        }
                    }
                    "in3" => {
                        if *gate > 0 {
                            get_col(trace, |row| &row.free_in_c, *gate - 1)
                        } else {
                            get_col_row(row0, |row| &row.free_in_c)
                        }
                    }
                    "out" => {
                        if *gate > 0 {
                            get_col(trace, |row| &row.free_in_d, *gate - 1)
                        } else {
                            get_col_row(row0, |row| &row.free_in_d)
                        }
                    }
                    _ => panic!("Invalid pin: {}", pin),
                },
                InputType::Input { bit, .. } => hash_input_bits[circuit][*bit],
                InputType::InputState { bit, .. } => state_bits[circuit][*bit],
            }
        }

        fn set_col<F: PrimeField64>(
            trace: &mut [Sha256fTraceRow<F>],
            cols: impl Fn(&mut Sha256fTraceRow<F>) -> &mut [F; CHUNKS_SHA256F],
            index: usize,
            value: u64,
        ) {
            let mut _value = value;
            let row = &mut trace[index];
            let cols = cols(row);
            for col in cols.iter_mut() {
                *col = F::from_u64(_value & MASK_BITS_SHA256F);
                _value >>= BITS_SHA256F;
            }
        }

        fn get_col<F: PrimeField64>(
            trace: &[Sha256fTraceRow<F>],
            cols: impl Fn(&Sha256fTraceRow<F>) -> &[F; CHUNKS_SHA256F],
            index: usize,
        ) -> u64 {
            let mut value = 0;
            let row = &trace[index];
            let cols = cols(row);
            for (i, col) in cols.iter().enumerate() {
                let col_i_val = F::as_canonical_u64(col);
                value += col_i_val << ((i * BITS_SHA256F) as u64);
            }
            value
        }

        fn get_col_row<F: PrimeField64>(
            trace_row: &Sha256fTraceRow<F>,
            cols: impl Fn(&Sha256fTraceRow<F>) -> &[F; CHUNKS_SHA256F],
        ) -> u64 {
            let mut value = 0;
            let row = trace_row;
            let cols = cols(row);
            for (i, col) in cols.iter().enumerate() {
                let col_i_val = F::as_canonical_u64(col);
                value += col_i_val << ((i * BITS_SHA256F) as u64);
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
        inputs: &[Vec<Sha256fInput>],
        trace_buffer: Option<Vec<F>>,
    ) -> AirInstance<F> {
        timer_start_trace!(SHA256F_TRACE);
        let mut sha256f_trace = if let Some(buffer) = trace_buffer {
            tracing::trace!("··· Using provided trace buffer");
            Sha256fTrace::new_from_vec_zeroes(buffer)
        } else {
            tracing::trace!("··· Creating new trace buffer");
            Sha256fTrace::new_zeroes()
        };
        let num_rows = sha256f_trace.num_rows();

        // Check that we can fit all the sha256fs in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_circuits_needed = num_inputs.div_ceil(NUM_SHA256F_PER_CIRCUIT);
        let num_rows_constants = 1; // Number of rows used for the constants
        let num_padding_rows = (num_rows - num_rows_constants) % self.circuit_size;
        let num_rows_needed =
            num_rows_constants + num_circuits_needed * self.circuit_size + num_padding_rows;

        // Sanity checks
        assert!(
            num_inputs <= self.num_available_sha256fs,
            "Exceeded available Sha256fs inputs: requested {}, but only {} are available.",
            num_inputs,
            self.num_available_sha256fs
        );
        assert!(num_circuits_needed <= self.num_available_circuits);
        assert!(num_rows_needed <= num_rows);

        tracing::info!(
            "··· Creating Sha256f instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        // Set a = 0b00..00, b = 0b11..11 and c = 0b00..00 at the first row
        // Set, e.g., the operation to be an XOR and set d = 0b11..11 = b = a ^ b ^ c
        let mut row: Sha256fTraceRow<F> = Default::default();
        let zeros = 0u64;
        let ones = MASK_BITS_SHA256F;
        let gate_op = self.sha256f_fixed[0].GATE_OP.as_canonical_u64();
        // Sanity check
        assert_eq!(gate_op, Sha256fTableGateOp::Xor as u64, "Invalid first row gate operation");
        for i in 0..CHUNKS_SHA256F {
            row.free_in_a[i] = F::from_u64(zeros);
            row.free_in_b[i] = F::from_u64(ones);
            row.free_in_c[i] = F::from_u64(zeros);
            row.free_in_d[i] = F::from_u64(ones);
        }

        // Assign the first row
        sha256f_trace[0] = row;

        // Update the multiplicity table
        let table_row =
            Sha256fTableSM::calculate_table_row(&Sha256fTableGateOp::Xor, zeros, ones, zeros);
        self.sha256f_table_sm.update_input(table_row, CHUNKS_SHA256F as u64);

        // Fill the rest of the trace
        // Flatten all the inputs, since I need to process them at least in chunks of NUM_SHA256F_PER_CIRCUIT
        let inputs = inputs.iter().flatten();
        self.process_trace(&mut sha256f_trace, num_rows_constants, inputs, num_inputs);
        timer_stop_and_log_trace!(SHA256F_TRACE);

        timer_start_trace!(SHA256F_PADDING);
        // A row with all zeros satisfies the constraints (assuming the operation to be XOR(0,0,0)=0)
        let padding_row: Sha256fTraceRow<F> = Default::default();
        for i in (num_rows_constants + self.circuit_size * self.num_available_circuits)..num_rows {
            let gate_op = self.sha256f_fixed[i].GATE_OP.as_canonical_u64();
            // Sanity check
            assert_eq!(
                gate_op,
                Sha256fTableGateOp::Xor as u64,
                "Invalid padding dummy gate operation"
            );

            let table_row = Sha256fTableSM::calculate_table_row(&Sha256fTableGateOp::Xor, 0, 0, 0);
            self.sha256f_table_sm.update_input(table_row, CHUNKS_SHA256F as u64);

            sha256f_trace[i] = padding_row;
        }
        timer_stop_and_log_trace!(SHA256F_PADDING);

        AirInstance::new_from_trace(FromTrace::new(&mut sha256f_trace))
    }
}
