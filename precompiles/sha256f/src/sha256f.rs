use core::panic;
use std::{fs, path::PathBuf, sync::Arc};

use generic_array::{typenum::U64, GenericArray};
use log::info;
use p3_field::PrimeField64;
use sha2::compress256;

use data_bus::{ExtOperationData, OperationBusData, OperationSha256Data, PayloadType};
use precompiles_common::MemBusHelpers;
use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{Sha256fFixed, Sha256fTrace, Sha256fTraceRow};

use crate::{sha256f_constants::*, InputType, Script, Sha256fTableGateOp, Sha256fTableSM};

use rayon::prelude::*;

/// The `Sha256fSM` struct encapsulates the logic of the Sha256f State Machine.
pub struct Sha256fSM {
    /// Reference to the Sha256f Table State Machine.
    sha256f_table_sm: Arc<Sha256fTableSM>,

    /// Script for the Sha256f's circuit representation
    script: Arc<Script>,

    /// Size of a circuit in the trace. It corresponds to the number of gates in the circuit.
    circuit_size: usize,

    /// Number of available circuits in the trace.
    num_available_circuits: usize,

    /// Number of available sha256fs in the trace.
    pub num_available_sha256fs: usize,
}

type Sha256fInput = [u64; INPUT_DATA_SIZE_BITS];

impl Sha256fSM {
    const MY_NAME: &'static str = "Sha256f ";

    /// Creates a new Sha256f State Machine instance.
    ///
    /// # Arguments
    /// * `sha256f_table_sm` - An `Arc`-wrapped reference to the Sha256f Table State Machine.
    ///
    /// # Returns
    /// A new `Sha256fSM` instance.
    pub fn new(sha256f_table_sm: Arc<Sha256fTableSM>, script_path: PathBuf) -> Arc<Self> {
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

        Arc::new(Self {
            sha256f_table_sm,
            script: Arc::new(script),
            circuit_size,
            num_available_circuits,
            num_available_sha256fs,
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
    pub fn process_slice<F: PrimeField64>(
        &self,
        fixed: &Sha256fFixed<F>,
        trace: &mut Sha256fTrace<F>,
        num_rows_constants: usize,
        inputs: &[OperationSha256Data<u64>],
    ) {
        let num_inputs = inputs.len();
        let mut inputs_bits: Vec<Sha256fInput> =
            vec![[0u64; INPUT_DATA_SIZE_BITS]; self.num_available_circuits];

        // Process the inputs
        let initial_offset = num_rows_constants;
        let input_offset = INPUT_SIZE; // Length of the input data
        inputs.iter().enumerate().for_each(|(i, input)| {
            let input_data = ExtOperationData::OperationSha256Data(*input);

            // Get the basic data from the input
            let step_received = OperationBusData::get_a(&input_data);
            let addr_received = OperationBusData::get_b(&input_data);

            // Get the raw sha256f input as INPUT_DATA_SIZE_U64 u64 values
            let sha256f_data: [u64; INPUT_DATA_SIZE_U64] =
                OperationBusData::get_extra_data(&input_data).try_into().unwrap();

            let circuit = i / NUM_SHA256F_PER_CIRCUIT;
            let circuit_pos = i % NUM_SHA256F_PER_CIRCUIT;
            let circuit_offset = circuit * self.circuit_size;

            // Update the multiplicity for the input
            let initial_pos = initial_offset + circuit_offset + circuit_pos;
            trace[initial_pos].multiplicity = F::ONE; // The pair (step_received, addr_received) is unique each time, so its multiplicity is 1

            // Process the sha256f input
            sha256f_data.iter().enumerate().for_each(|(j, &value)| {
                let chunk_offset = j * RB_SIZE;
                let pos = initial_pos + chunk_offset;

                // At the beginning of each 64-bit chunk, we set the step and address
                trace[pos].step = F::from_u64(step_received);
                trace[pos].addr = F::from_u64(addr_received + 8 * j as u64);
                trace[pos].is_val = F::ONE;

                // Process the 64-bit chunk
                for k in 0..64 {
                    // Divide the value in bits:
                    //    (circuit i) [0b1011,  0b0011,  0b1000,  0b0010]
                    //    (circuit i) [1,0,1,1, 0,0,1,1, 1,0,0,0, 0,0,1,0]
                    let bit_pos = k + 64 * j;
                    let old_value = inputs_bits[circuit][bit_pos];
                    let new_bit = (value >> (63 - k)) & 1;
                    inputs_bits[circuit][bit_pos] = (new_bit << circuit_pos) | old_value;

                    // We update bit[i] and val[i]
                    let bit_pos = k % BITS_IN_PARALLEL;
                    let bit_offset = (k - bit_pos) * NUM_SHA256F_PER_CIRCUIT / BITS_IN_PARALLEL;
                    update_bit_val(fixed, trace, pos + bit_offset, new_bit, circuit_pos, bit_pos);
                }
            });

            // Apply the sha256f function
            let mut sha256f_state: [u64; 4] = sha256f_data[..4].try_into().unwrap();
            let sha256f_input: [u64; 8] = sha256f_data[4..].try_into().unwrap();
            let mut state_u32 = convert_u64_to_u32_be_words(&sha256f_state);
            let block: GenericArray<u8, U64> = u64s_to_generic_array_be(&sha256f_input);
            let blocks = &[block];
            compress256(&mut state_u32, blocks);
            sha256f_state = convert_u32s_back_to_u64_be(&state_u32);

            // Process the output
            sha256f_state.iter().enumerate().for_each(|(j, &value)| {
                let chunk_offset = j * RB_SIZE;
                let pos = initial_pos + input_offset + chunk_offset;

                // At the beginning of each 64-bit chunk, we set the step and address
                trace[pos].step = F::from_u64(step_received);
                trace[pos].addr = F::from_u64(addr_received + 8 * j as u64);
                trace[pos].is_val = F::ONE;

                // Process the 64-bit chunk
                for k in 0..64 {
                    // We update bit[i] and val[i]
                    let new_bit = (value >> (63 - k)) & 1;
                    let bit_pos = k % BITS_IN_PARALLEL;
                    let bit_offset = (k - bit_pos) * NUM_SHA256F_PER_CIRCUIT / BITS_IN_PARALLEL;
                    update_bit_val(fixed, trace, pos + bit_offset, new_bit, circuit_pos, bit_pos);
                }
            });

            // At the end of the outputs, we set the next step and address for the constraints to be satisfied
            let final_pos = initial_pos + input_offset + (sha256f_state.len() - 1) * RB_SIZE;
            trace[final_pos + RB_SIZE].step = trace[final_pos].step;
            trace[final_pos + RB_SIZE].addr = trace[final_pos].addr
        });

        // It the number of inputs is less than the available sha256fs, we need to fill the remaining inputs
        if num_inputs < self.num_available_sha256fs {
            // Compute the hash of zero
            let mut zero_state = [0u32; 8];
            let block_zeros: GenericArray<u8, U64> = GenericArray::default();
            let blocks_zeros = &[block_zeros];
            compress256(&mut zero_state, blocks_zeros);
            // hash_of_0: [0x7ca51614425c3ba8, 0xce54dd2fc2020ae7, 0xb6e574d198136d0f, 0xae7e26ccbf0be7a6]
            let zero_state: [u64; 4] = convert_u32s_back_to_u64_be(&zero_state);

            // If the number of inputs is not a multiple of NUM_SHA256F_PER_CIRCUIT,
            // we fill the last processed circuit
            let rem_inputs = num_inputs % NUM_SHA256F_PER_CIRCUIT;
            if rem_inputs != 0 {
                let last_circuit = (num_inputs - 1) / NUM_SHA256F_PER_CIRCUIT;
                let circuit_offset = last_circuit * self.circuit_size;
                // Since no more bits are being introduced as input, we let 0 be the
                // new bits and therefore we repeat the last values
                for j in 0..INPUT_DATA_SIZE_BITS / BITS_IN_PARALLEL {
                    let block_offset = j * NUM_SHA256F_PER_CIRCUIT;
                    for k in rem_inputs..NUM_SHA256F_PER_CIRCUIT {
                        let pos = initial_offset + circuit_offset + block_offset + k;
                        for l in 0..BITS_IN_PARALLEL {
                            // trace[pos+1].bit[l] = F::ZERO;
                            trace[pos + 1].val[l] = trace[pos].val[l];
                        }
                    }
                }

                let initial_pos = initial_offset + circuit_offset;
                // Since the new bits are all zero, we have to set the hash of 0 as the respective output
                zero_state.iter().enumerate().for_each(|(j, &value)| {
                    let chunk_offset = j * RB_SIZE;
                    let pos = initial_pos + input_offset + chunk_offset;
                    for k in 0..64 {
                        let new_bit = (value >> (63 - k)) & 1;
                        let bit_pos = k % BITS_IN_PARALLEL;
                        let bit_offset = (k - bit_pos) * NUM_SHA256F_PER_CIRCUIT / BITS_IN_PARALLEL;
                        for w in rem_inputs..NUM_SHA256F_PER_CIRCUIT {
                            update_bit_val(fixed, trace, pos + bit_offset + w, new_bit, w, bit_pos);
                        }
                    }
                });
            }

            // Fill the remaining circuits with the hash of 0
            let next_circuit = num_inputs.div_ceil(NUM_SHA256F_PER_CIRCUIT);
            zero_state.iter().enumerate().for_each(|(j, &value)| {
                for s in next_circuit..self.num_available_circuits {
                    let circuit_offset = s * self.circuit_size;
                    let chunk_offset = j * RB_SIZE;
                    for k in 0..64 {
                        let new_bit = (value >> (63 - k)) & 1;
                        let bit_pos = k % BITS_IN_PARALLEL;
                        let bit_offset = (k - bit_pos) * NUM_SHA256F_PER_CIRCUIT / BITS_IN_PARALLEL;
                        let pos = initial_offset
                            + circuit_offset
                            + input_offset
                            + chunk_offset
                            + bit_offset;
                        for w in 0..NUM_SHA256F_PER_CIRCUIT {
                            update_bit_val(fixed, trace, pos + w, new_bit, w, bit_pos);
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

        let row0 = trace.buffer[0];

        let mut trace_slice = &mut trace.buffer[1..];
        let mut par_traces = Vec::new();

        for _ in 0..self.num_available_circuits {
            let take = self.circuit_size.min(trace_slice.len());
            let (head, tail) = trace_slice.split_at_mut(take);
            par_traces.push(head);
            trace_slice = tail;
        }

        let program = &self.script.program;
        par_traces.into_par_iter().enumerate().for_each(|(i, par_trace)| {
            for j in 0..self.circuit_size {
                let line = &program[j];
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
            fixed: &Sha256fFixed<F>,
            trace: &mut Sha256fTrace<F>,
            pos: usize,
            new_bit: u64,
            circuit_pos: usize,
            bit_pos: usize,
        ) {
            trace[pos].bit[bit_pos] = F::from_u64(new_bit);
            trace[pos + 1].val[bit_pos] = if fixed[pos].latch_num_sha256f == F::ZERO {
                trace[pos].val[bit_pos] + F::from_u64(new_bit << circuit_pos)
            } else {
                F::from_u64(new_bit << circuit_pos)
            };
        }

        fn get_val<F: PrimeField64>(
            trace: &[Sha256fTraceRow<F>],
            row0: &Sha256fTraceRow<F>,
            state_bits: &Vec<[u64; STATE_SIZE_BITS]>,
            hash_input_bits: &Vec<[u64; INPUT_SIZE_BITS]>,
            circuit: usize,
            gate_input: &InputType,
        ) -> u64 {
            match gate_input {
                InputType::Wired { gate, pin, .. } => match pin.as_str() {
                    "in1" => {
                        if *gate > 0 {
                            get_col(trace, |row| &row.free_in_a, *gate - 1)
                        } else {
                            get_col_row(&row0, |row| &row.free_in_a)
                        }
                    }
                    "in2" => {
                        if *gate > 0 {
                            get_col(trace, |row| &row.free_in_b, *gate - 1)
                        } else {
                            get_col_row(&row0, |row| &row.free_in_b)
                        }
                    }
                    "in3" => {
                        if *gate > 0 {
                            get_col(trace, |row| &row.free_in_c, *gate - 1)
                        } else {
                            get_col_row(&row0, |row| &row.free_in_c)
                        }
                    }
                    "out" => {
                        if *gate > 0 {
                            get_col(trace, |row| &row.free_in_d, *gate - 1)
                        } else {
                            get_col_row(&row0, |row| &row.free_in_d)
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
    pub fn compute_witness<F: PrimeField64>(
        &self,
        sctx: &SetupCtx<F>,
        inputs: &[Vec<OperationSha256Data<u64>>],
    ) -> AirInstance<F> {
        // Get the fixed cols
        let airgroup_id = Sha256fTrace::<usize>::AIRGROUP_ID;
        let air_id = Sha256fTrace::<usize>::AIR_ID;
        let fixed_pols = sctx.get_fixed(airgroup_id, air_id);
        let fixed = Sha256fFixed::from_vec(fixed_pols);

        timer_start_trace!(SHA256F_TRACE);
        let mut sha256f_trace = Sha256fTrace::new();
        let num_rows = sha256f_trace.num_rows();

        // Flatten the inputs
        let inputs: Vec<OperationSha256Data<u64>> = inputs.iter().flatten().cloned().collect();

        // Check that we can fit all the sha256fs in the trace
        let num_inputs: usize = inputs.len();
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

        info!(
            "{}: ··· Creating Sha256f instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        // Set a = 0b00..00, b = 0b11..11 and c = 0b00..00 at the first row
        // Set, e.g., the operation to be an XOR and set d = 0b11..11 = b = a ^ b ^ c
        let mut row: Sha256fTraceRow<F> = Default::default();
        let zeros = 0u64;
        let ones = MASK_BITS_SHA256F;
        let gate_op = fixed[0].GATE_OP.as_canonical_u64();
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
        self.process_slice(&fixed, &mut sha256f_trace, num_rows_constants, &inputs);
        timer_stop_and_log_trace!(SHA256F_TRACE);

        timer_start_trace!(SHA256F_PADDING);
        // A row with all zeros satisfies the constraints (assuming the operation to be XOR(0,0,0)=0)
        let padding_row: Sha256fTraceRow<F> = Default::default();
        for i in (num_rows_constants + self.circuit_size * self.num_available_circuits)..num_rows {
            let gate_op = fixed[i].GATE_OP.as_canonical_u64();
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

    /// Generates memory inputs.
    pub fn generate_inputs(
        input: &OperationSha256Data<u64>,
        counters_mode: bool,
    ) -> Vec<Vec<PayloadType>> {
        // Get the basic data from the input
        let input_data = ExtOperationData::OperationSha256Data(*input);

        let step_main = OperationBusData::get_a(&input_data);
        let addr = OperationBusData::get_b(&input_data) as u32;

        let mut mem_data = vec![];
        if counters_mode {
            // On counter phase we don't need final values, we only need the
            // address and step
            // Compute the reads
            for i in 0..INPUT_DATA_SIZE_U64 {
                let new_addr = addr + 8 * i as u32;
                let read = MemBusHelpers::mem_aligned_load(new_addr, step_main, 0);
                mem_data.push(read.to_vec());
            }

            // Compute the writes
            for i in 0..INPUT_DATA_SIZE_U64 {
                let new_addr = addr + 8 * i as u32;
                let write = MemBusHelpers::mem_aligned_write(new_addr, step_main, 0);
                mem_data.push(write.to_vec());
            }

            return mem_data;
        }

        // Get the raw sha256f input as INPUT_DATA_SIZE_U64 u64 values
        let sha256f_data: [u64; INPUT_DATA_SIZE_U64] =
            OperationBusData::get_extra_data(&input_data).try_into().unwrap();

        // Compute the reads
        for (i, &input) in sha256f_data.iter().enumerate() {
            let new_addr = addr + 8 * i as u32;
            let read = MemBusHelpers::mem_aligned_load(new_addr, step_main, input);
            mem_data.push(read.to_vec());
        }

        // Apply the sha256f function and get the output
        let mut sha256f_state: [u64; 4] = sha256f_data[..4].try_into().unwrap();
        let sha256f_input: [u64; 8] = sha256f_data[4..].try_into().unwrap();
        let mut state_u32 = convert_u64_to_u32_be_words(&sha256f_state);
        let block: GenericArray<u8, U64> = u64s_to_generic_array_be(&sha256f_input);
        let blocks = &[block];
        compress256(&mut state_u32, blocks);
        sha256f_state = convert_u32s_back_to_u64_be(&state_u32);

        // Compute the writes
        for (i, &output) in sha256f_state.iter().enumerate() {
            let new_addr = addr + 8 * i as u32;
            let write = MemBusHelpers::mem_aligned_write(new_addr, step_main, output);
            mem_data.push(write.to_vec());
        }

        mem_data
    }
}

fn convert_u64_to_u32_be_words(input: &[u64; 4]) -> [u32; 8] {
    let mut out = [0u32; 8];
    for (i, &word) in input.iter().enumerate() {
        let bytes = word.to_be_bytes();
        out[2 * i] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        out[2 * i + 1] = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    }
    out
}

fn u64s_to_generic_array_be(input: &[u64; 8]) -> GenericArray<u8, U64> {
    let mut out = [0u8; 64];
    for (i, word) in input.iter().enumerate() {
        let bytes = word.to_be_bytes();
        out[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }
    GenericArray::<u8, U64>::clone_from_slice(&out)
}

fn convert_u32s_back_to_u64_be(words: &[u32; 8]) -> [u64; 4] {
    let mut out = [0u64; 4];
    for i in 0..4 {
        let high = words[2 * i].to_be_bytes();
        let low = words[2 * i + 1].to_be_bytes();
        out[i] = u64::from_be_bytes([
            high[0], high[1], high[2], high[3], low[0], low[1], low[2], low[3],
        ]);
    }
    out
}
