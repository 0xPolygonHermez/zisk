use core::panic;
use std::sync::Arc;

use log::info;
use p3_field::PrimeField64;

use data_bus::{ExtOperationData, OperationArith256Data, OperationBusData, PayloadType};
use precompiles_common::MemBusHelpers;
use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{Arith256Trace, Arith256TraceRow};

use crate::arith_256_constants::*;

/// The `Arith256SM` struct encapsulates the logic of the Arith256 State Machine.
pub struct Arith256SM {
    /// Number of available arith256s in the trace.
    pub num_available_arith256s: usize,
}

impl Arith256SM {
    const MY_NAME: &'static str = "Arith256 ";

    /// Creates a new Arith256 State Machine instance.
    ///
    /// # Arguments
    /// * `arith256_table_sm` - An `Arc`-wrapped reference to the Arith256 Table State Machine.
    ///
    /// # Returns
    /// A new `Arith256SM` instance.
    pub fn new() -> Arc<Self> {
        // Compute some useful values
        let num_available_arith256s = Arith256Trace::<usize>::NUM_ROWS / ARITH_256_ROWS_BY_OP;

        Arc::new(Self { num_available_arith256s })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Arith256 trace.
    /// * `num_slots` - The number of slots to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    /*    #[inline(always)]
        pub fn process_slice<F: PrimeField64>(
            &self,
            fixed: &Arith256Fixed<F>,
            trace: &mut Arith256Trace<F>,
            num_rows_constants: usize,
            inputs: &[OperationArith256Data<u64>],
        ) {
            let num_inputs = inputs.len();
            let mut inputs_bits: Vec<Arith256Input> =
                vec![[0u64; INPUT_DATA_SIZE_BITS]; self.num_available_slots];

            // Process the inputs
            let initial_offset = num_rows_constants;
            let input_offset = Self::BLOCKS_PER_SLOT / BITS_IN_PARALLEL_ARITH_256; // Length of the input data
            inputs.iter().enumerate().for_each(|(i, input)| {
                let input_data = ExtOperationData::OperationArith256Data(*input);

                // Get the basic data from the input
                let step_received = OperationBusData::get_a(&input_data);
                let addr_received = OperationBusData::get_b(&input_data);

                // Get the raw arith256 input as 25 u64 values
                let arith256_input: [u64; 25] =
                    OperationBusData::get_extra_data(&input_data).try_into().unwrap();

                let slot = i / Self::NUM_ARITH_256_PER_SLOT;
                let slot_pos = i % Self::NUM_ARITH_256_PER_SLOT;
                let slot_offset = slot * self.slot_size;

                // Update the multiplicity for the input
                let initial_pos = initial_offset + slot_offset + slot_pos;
                trace[initial_pos].multiplicity = F::one(); // The pair (step_received, addr_received) is unique each time, so its multiplicity is 1

                // Process the arith256 input
                arith256_input.iter().enumerate().for_each(|(j, &value)| {
                    let chunk_offset = j * Self::RB_SIZE;
                    let pos = initial_pos + chunk_offset;

                    // At the beginning of each 64-bit chunk, we set the step and address
                    trace[pos].step = F::from_canonical_u64(step_received);
                    trace[pos].addr = F::from_canonical_u64(addr_received + 8 * j as u64);
                    trace[pos].is_val = F::one();

                    // Process the 64-bit chunk
                    for k in 0..64 {
                        // Divide the value in bits:
                        //    (slot i) [0b1011,  0b0011,  0b1000,  0b0010]
                        //    (slot i) [1,1,0,1, 1,1,0,0, 0,0,0,1, 0,0,1,0]
                        let bit_pos = k + 64 * j;
                        let old_value = inputs_bits[slot][bit_pos];
                        let new_bit = (value >> k) & 1;
                        inputs_bits[slot][bit_pos] = (new_bit << slot_pos) | old_value;

                        // We update bit[i] and val[i]
                        let bit_pos = k % BITS_IN_PARALLEL_ARITH_256;
                        let bit_offset =
                            (k - bit_pos) * Self::NUM_ARITH_256_PER_SLOT / BITS_IN_PARALLEL_ARITH_256;
                        update_bit_val(fixed, trace, pos + bit_offset, new_bit, slot_pos, bit_pos);
                    }
                });

                // Apply the arith256 function and get the output
                let mut arith256_output = arith256_input;
                arith256(&mut arith256_output);

                // Process the output
                arith256_output.iter().enumerate().for_each(|(j, &value)| {
                    let chunk_offset = j * Self::RB_SIZE;
                    let pos = initial_pos + input_offset + chunk_offset;

                    // At the beginning of each 64-bit chunk, we set the step and address
                    trace[pos].step = F::from_canonical_u64(step_received);
                    trace[pos].addr = F::from_canonical_u64(addr_received + 8 * j as u64);
                    trace[pos].is_val = F::one();

                    // Process the 64-bit chunk
                    for k in 0..64 {
                        // We update bit[i] and val[i]
                        let new_bit = (value >> k) & 1;
                        let bit_pos = k % BITS_IN_PARALLEL_ARITH_256;
                        let bit_offset =
                            (k - bit_pos) * Self::NUM_ARITH_256_PER_SLOT / BITS_IN_PARALLEL_ARITH_256;
                        update_bit_val(fixed, trace, pos + bit_offset, new_bit, slot_pos, bit_pos);
                    }
                });

                // At the end of the outputs, we set the next step and address for the constraints to be satisfied
                let final_pos =
                    initial_pos + input_offset + (arith256_output.len() - 1) * Self::RB_SIZE;
                trace[final_pos + Self::RB_SIZE].step = trace[final_pos].step;
                trace[final_pos + Self::RB_SIZE].addr = trace[final_pos].addr
            });

            // It the number of inputs is less than the available arith256s, we need to fill the remaining inputs
            if num_inputs < self.num_available_arith256s {
                // Compute the hash of zero
                let mut zero_output: [u64; 25] = [0u64; 25];
                arith256(&mut zero_output);

                // If the number of inputs is not a multiple of NUM_ARITH_256_PER_SLOT,
                // we fill the last processed slot
                let rem_inputs = num_inputs % Self::NUM_ARITH_256_PER_SLOT;
                if rem_inputs != 0 {
                    let last_slot = (num_inputs - 1) / Self::NUM_ARITH_256_PER_SLOT;
                    let slot_offset = last_slot * self.slot_size;
                    // Since no more bits are being introduced as input, we let 0 be the
                    // new bits and therefore we repeat the last values
                    for j in 0..RB * RB_BLOCKS_TO_PROCESS / 2 {
                        let block_offset = j * Self::NUM_ARITH_256_PER_SLOT;
                        for k in rem_inputs..Self::NUM_ARITH_256_PER_SLOT {
                            let pos = initial_offset + slot_offset + block_offset + k;
                            for l in 0..BITS_IN_PARALLEL_ARITH_256 {
                                // trace[pos+1].bit[l] = F::zero();
                                trace[pos + 1].val[l] = trace[pos].val[l];
                            }
                        }
                    }

                    let initial_pos = initial_offset + slot_offset;
                    // Since the new bits are all zero, we have to set the hash of 0 as the respective output
                    zero_output.iter().enumerate().for_each(|(j, &value)| {
                        let chunk_offset = j * Self::RB_SIZE;
                        let pos = initial_pos + input_offset + chunk_offset;
                        for k in 0..64 {
                            let new_bit = (value >> k) & 1;
                            let bit_pos = k % BITS_IN_PARALLEL_ARITH_256;
                            let bit_offset = (k - bit_pos) * Self::NUM_ARITH_256_PER_SLOT
                                / BITS_IN_PARALLEL_ARITH_256;
                            for w in rem_inputs..Self::NUM_ARITH_256_PER_SLOT {
                                update_bit_val(fixed, trace, pos + bit_offset + w, new_bit, w, bit_pos);
                            }
                        }
                    });
                }

                // Fill the remaining slots with the hash of 0
                let next_slot = num_inputs.div_ceil(Self::NUM_ARITH_256_PER_SLOT);
                zero_output.iter().enumerate().for_each(|(j, &value)| {
                    for s in next_slot..self.num_available_slots {
                        let slot_offset = s * self.slot_size;
                        let chunk_offset = j * Self::RB_SIZE;
                        for k in 0..64 {
                            let new_bit = (value >> k) & 1;
                            let bit_pos = k % BITS_IN_PARALLEL_ARITH_256;
                            let bit_offset = (k - bit_pos) * Self::NUM_ARITH_256_PER_SLOT
                                / BITS_IN_PARALLEL_ARITH_256;
                            let pos =
                                initial_offset + slot_offset + input_offset + chunk_offset + bit_offset;
                            for w in 0..Self::NUM_ARITH_256_PER_SLOT {
                                update_bit_val(fixed, trace, pos + w, new_bit, w, bit_pos);
                            }
                        }
                    }
                });
            }

            // Set the values of free_in_a, free_in_b, free_in_c using the script
            let script = self.script.clone();
            let mut offset = 0;
            for (i, input) in inputs_bits.iter().enumerate() {
                let mut bit_input_pos = [0u64; INPUT_DATA_SIZE_BITS];
                let mut bit_output_pos = [0u64; INPUT_DATA_SIZE_BITS];
                for j in 0..self.slot_size {
                    let line = &script.program[j];
                    let row = line.ref_ + i * self.slot_size;

                    let a = &line.a;
                    match a {
                        ValueType::Input(a) => {
                            set_col(trace, |row| &mut row.free_in_a, row, input[a.bit]);
                        }
                        ValueType::Wired(b) => {
                            let mut gate = b.gate;
                            if gate > 0 {
                                gate += offset;
                            }

                            let pin = &b.pin;
                            if pin == "a" {
                                let pinned_value = get_col(trace, |row| &mut row.free_in_a, gate);
                                set_col(trace, |row| &mut row.free_in_a, row, pinned_value);
                            } else if pin == "b" {
                                let pinned_value = get_col(trace, |row| &mut row.free_in_b, gate);
                                set_col(trace, |row| &mut row.free_in_a, row, pinned_value);
                            } else if pin == "c" {
                                let pinned_value = get_col(trace, |row| &mut row.free_in_c, gate);
                                set_col(trace, |row| &mut row.free_in_a, row, pinned_value);
                            } else {
                                panic!("Invalid pin");
                            }
                        }
                    }

                    let b = &line.b;
                    match b {
                        ValueType::Input(b) => {
                            set_col(trace, |row| &mut row.free_in_b, row, input[b.bit]);
                        }
                        ValueType::Wired(b) => {
                            let mut gate = b.gate;
                            if gate > 0 {
                                gate += offset;
                            }

                            let pin = &b.pin;
                            if pin == "a" {
                                let pinned_value = get_col(trace, |row| &mut row.free_in_a, gate);
                                set_col(trace, |row| &mut row.free_in_b, row, pinned_value);
                            } else if pin == "b" {
                                let pinned_value = get_col(trace, |row| &mut row.free_in_b, gate);
                                set_col(trace, |row| &mut row.free_in_b, row, pinned_value);
                            } else if pin == "c" {
                                let pinned_value = get_col(trace, |row| &mut row.free_in_c, gate);
                                set_col(trace, |row| &mut row.free_in_b, row, pinned_value);
                            } else {
                                panic!("Invalid pin");
                            }
                        }
                    }

                    let a_val =
                        get_col(trace, |row| &mut row.free_in_a, row) & MASK_CHUNK_BITS_ARITH_256;
                    let b_val =
                        get_col(trace, |row| &mut row.free_in_b, row) & MASK_CHUNK_BITS_ARITH_256;
                    let op = &line.op;
                    let c_val;
                    if op == "xor" {
                        c_val = a_val ^ b_val;
                    } else if op == "andp" {
                        c_val = (a_val ^ MASK_CHUNK_BITS_ARITH_256) & b_val
                    } else {
                        panic!("Invalid operation");
                    }

                    set_col(trace, |row| &mut row.free_in_c, row, c_val);

                    if (line.ref_ >= STATE_IN_REF_0)
                        && (line.ref_
                            <= STATE_IN_REF_0
                                + (INPUT_DATA_SIZE_BITS - 2) * STATE_IN_REF_DISTANCE / 2
                                + 1)
                        && ((line.ref_ - STATE_IN_REF_0) % STATE_IN_REF_DISTANCE < 2)
                    {
                        let ref_pos = line.ref_ - STATE_IN_REF_0;
                        let bit_pos = ref_pos / STATE_IN_REF_DISTANCE * 2 + ref_pos % 2;
                        bit_input_pos[bit_pos] = a_val;
                    }

                    if (line.ref_ >= STATE_OUT_REF_0)
                        && (line.ref_
                            <= STATE_OUT_REF_0
                                + (INPUT_DATA_SIZE_BITS - 2) * STATE_OUT_REF_DISTANCE / 2
                                + 1)
                        && ((line.ref_ - STATE_OUT_REF_0) % STATE_OUT_REF_DISTANCE < 2)
                    {
                        let ref_pos = line.ref_ - STATE_OUT_REF_0;
                        let bit_pos = ref_pos / STATE_OUT_REF_DISTANCE * 2 + ref_pos % 2;
                        bit_output_pos[bit_pos] = a_val;
                    }
                }

                // Update the multiplicity table for the slot
                let row_idx = if offset == 0 { 1 } else { offset + 1 };
                for i in row_idx..(row_idx + self.slot_size) {
                    let a = trace[i].free_in_a;
                    let b = trace[i].free_in_b;
                    let gate_op = fixed[i].GATE_OP;
                    let gate_op_val = match F::as_canonical_u64(&gate_op) {
                        0u64 => Arith256TableGateOp::Xor,
                        1u64 => Arith256TableGateOp::Andp,
                        _ => panic!("Invalid gate operation"),
                    };
                    for j in 0..CHUNKS_ARITH_256 {
                        let a_val = F::as_canonical_u64(&a[j]);
                        let b_val = F::as_canonical_u64(&b[j]);
                        let table_row =
                            Arith256TableSM::calculate_table_row(&gate_op_val, a_val, b_val);
                        self.arith256_table_sm.update_input(table_row, 1);
                    }
                }

                // Move to the next slot
                offset += self.slot_size;
            }

            fn update_bit_val<F: PrimeField64>(
                fixed: &Arith256Fixed<F>,
                trace: &mut Arith256Trace<F>,
                pos: usize,
                new_bit: u64,
                slot_pos: usize,
                bit_pos: usize,
            ) {
                trace[pos].bit[bit_pos] = F::from_canonical_u64(new_bit);
                trace[pos + 1].val[bit_pos] = if fixed[pos].latch_num_arith256 == F::zero() {
                    trace[pos].val[bit_pos] + F::from_canonical_u64(new_bit << slot_pos)
                } else {
                    F::from_canonical_u64(new_bit << slot_pos)
                };
            }

            fn set_col<F: PrimeField64>(
                trace: &mut Arith256Trace<F>,
                cols: impl Fn(&mut Arith256TraceRow<F>) -> &mut [F; CHUNKS_ARITH_256],
                index: usize,
                value: u64,
            ) {
                let mut _value = value;
                let row = &mut trace[index];
                let cols = cols(row);
                for col in cols.iter_mut() {
                    *col = F::from_canonical_u64(_value & MASK_BITS_ARITH_256);
                    _value >>= BITS_ARITH_256;
                }
            }

            fn get_col<F: PrimeField64>(
                trace: &mut Arith256Trace<F>,
                cols: impl Fn(&mut Arith256TraceRow<F>) -> &mut [F; CHUNKS_ARITH_256],
                index: usize,
            ) -> u64 {
                let mut value = 0;
                let row = &mut trace[index];
                let cols = cols(row);
                for (i, col) in cols.iter().enumerate() {
                    let col_i_val = F::as_canonical_u64(col);
                    value += col_i_val << ((i * BITS_ARITH_256) as u64);
                }
                value
            }
        }
    */
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
        inputs: &[Vec<OperationArith256Data<u64>>],
    ) -> AirInstance<F> {
        // Get the fixed cols
        let airgroup_id = Arith256Trace::<usize>::AIRGROUP_ID;
        let air_id = Arith256Trace::<usize>::AIR_ID;

        unimplemented!()
    }
}
