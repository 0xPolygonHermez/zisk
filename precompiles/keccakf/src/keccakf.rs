use core::panic;
use std::{fs, sync::Arc};

use log::info;
use p3_field::PrimeField64;

use tiny_keccak::keccakf;

use data_bus::{ExtOperationData, OperationBusData, OperationKeccakData, PayloadType};
use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{KeccakfFixed, KeccakfTableTrace, KeccakfTrace, KeccakfTraceRow};

use crate::{keccakf_constants::*, KeccakfTableGateOp, KeccakfTableSM, Script, ValueType};

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM {
    /// Reference to the Keccakf Table State Machine.
    keccakf_table_sm: Arc<KeccakfTableSM>,

    /// Script for the Keccakf's circuit representation
    script: Arc<Script>,

    /// Size of a slot in the trace. It corresponds to the number of gates in the circuit.
    slot_size: usize,

    /// Number of available slots in the trace.
    num_available_slots: usize,

    /// Number of available keccakfs in the trace.
    num_available_keccakfs: usize,
}

type KeccakfInput = [u64; INPUT_DATA_SIZE_BITS];

impl KeccakfSM {
    const MY_NAME: &'static str = "Keccakf ";

    const NUM_KECCAKF_PER_SLOT: usize = CHUNKS_KECCAKF * BITS_KECCAKF;

    const BLOCKS_PER_SLOT: usize = Self::NUM_KECCAKF_PER_SLOT * RB * RB_BLOCKS_TO_PROCESS;

    /// Creates a new Keccakf State Machine instance.
    ///
    /// # Arguments
    /// * `keccakf_table_sm` - An `Arc`-wrapped reference to the Keccakf Table State Machine.
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new(keccakf_table_sm: Arc<KeccakfTableSM>) -> Arc<Self> {
        // Parse the script
        let script = fs::read_to_string("../zisk/precompiles/keccakf/src/keccakf_script.json")
            .expect("Failed to read keccakf_script.json");
        let script: Script =
            serde_json::from_str(&script).expect("Failed to parse keccakf_script.json");
        let slot_size = script.maxref;

        // Check that the script is valid
        assert!(script.xors + script.andps == slot_size);
        assert!(script.program.len() == slot_size);

        // Compute some useful values
        let num_available_slots = (KeccakfTrace::<usize>::NUM_ROWS - 1) / slot_size;
        let num_available_keccakfs = Self::NUM_KECCAKF_PER_SLOT * num_available_slots;

        Arc::new(Self {
            keccakf_table_sm,
            script: Arc::new(script),
            slot_size,
            num_available_slots,
            num_available_keccakfs,
        })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Keccakf trace.
    /// * `num_slots` - The number of slots to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_slice<F: PrimeField64>(
        &self,
        fixed: &KeccakfFixed<F>,
        trace: &mut KeccakfTrace<F>,
        inputs: &[OperationKeccakData<u64>],
        multiplicity: &mut [u64],
    ) {
        let num_inputs = inputs.len();
        let mut inputs_bits: Vec<KeccakfInput> =
            vec![[0u64; INPUT_DATA_SIZE_BITS]; self.num_available_slots];

        // Process the inputs
        let initial_offset = 1; // Number of constant values used in the circuit
        let input_offset = Self::BLOCKS_PER_SLOT / 2;
        inputs.iter().enumerate().for_each(|(i, input)| {
            // Get the basic data from the input
            let step_received =
                OperationBusData::get_a(&ExtOperationData::OperationKeccakData(*input));
            let addr_received =
                OperationBusData::get_b(&ExtOperationData::OperationKeccakData(*input));

            // Get the raw keccakf input as 25 u64 values
            let keccakf_input: [u64; 25] =
                OperationBusData::get_extra_data(&ExtOperationData::OperationKeccakData(*input))
                    .try_into()
                    .unwrap();

            // Apply the keccakf function and get the output
            let mut keccakf_output = keccakf_input.clone();
            keccakf(&mut keccakf_output);

            // Process the keccakf input
            let slot = i / Self::NUM_KECCAKF_PER_SLOT;
            let slot_pos = i % Self::NUM_KECCAKF_PER_SLOT;
            let slot_offset = slot * self.slot_size;
            keccakf_input.iter().enumerate().for_each(|(j, &value)| {
                let chunk_offset = j * Self::NUM_KECCAKF_PER_SLOT * 64 / 2;
                for k in 0..64 {
                    // Divide the value in bits:
                    //    (slot i) [0b1011,  0b0011,  0b1000,  0b0010]
                    //    (slot i) [1,1,0,1, 1,1,0,0, 0,0,0,1, 0,0,1,0]
                    let bit_pos = k + 64 * j;
                    let old_value = inputs_bits[slot][bit_pos];
                    let new_bit = (value >> k) & 1;
                    inputs_bits[slot][bit_pos] = (old_value << 1) | new_bit;

                    // In even bits, we update bit1 and val1; in odd bits, we update bit2 and val2
                    if k % 2 == 0 {
                        let bit_offset = k * Self::NUM_KECCAKF_PER_SLOT / 2;
                        let pos = initial_offset + slot_offset + chunk_offset + bit_offset;
                        update_bit_val(fixed, trace, pos, new_bit, slot_pos, true);

                        // We use the even bits to activate also set the step and addr values
                        trace[pos].step = F::from_canonical_u64(step_received);
                        trace[pos].addr = F::from_canonical_u64(addr_received);
                        trace[pos].is_val = F::one();

                        // TODO: Write the raw input/output to memory??
                    } else {
                        let bit_offset = (k - 1) * Self::NUM_KECCAKF_PER_SLOT / 2;
                        let pos = initial_offset + slot_offset + chunk_offset + bit_offset;
                        update_bit_val(fixed, trace, pos, new_bit, slot_pos, false);
                    }
                }
            });

            // Process the output
            keccakf_output.iter().enumerate().for_each(|(j, &value)| {
                let chunk_offset = j * Self::NUM_KECCAKF_PER_SLOT * 64 / 2;
                for k in 0..64 {
                    let new_bit = (value >> k) & 1;
                    if k % 2 == 0 {
                        let bit_offset = k * Self::NUM_KECCAKF_PER_SLOT / 2;
                        let pos =
                            initial_offset + slot_offset + input_offset + chunk_offset + bit_offset;
                        update_bit_val(fixed, trace, pos, new_bit, slot_pos, true);

                        trace[pos].step = F::from_canonical_u64(step_received);
                        trace[pos].addr = F::from_canonical_u64(addr_received);
                        if j == 24 && k < 62 {
                            trace[pos].is_val = F::one();
                        }

                        // TODO: Write the raw input/output to memory??
                    } else {
                        let bit_offset = (k - 1) * Self::NUM_KECCAKF_PER_SLOT / 2;
                        let pos =
                            initial_offset + slot_offset + input_offset + chunk_offset + bit_offset;
                        update_bit_val(fixed, trace, pos, new_bit, slot_pos, false);
                    }
                }
            });

            // Update the multiplicity for the input
            let pos = initial_offset + slot_offset + slot_pos;
            trace[pos].multiplicity = F::one(); // The pair (step_input, addr_input) is unique each time, so its multiplicity is 1
        });
        // println!("\nInput (P): {:?}", print_seq_format(&inputs_bits[0]));

        // It the number of inputs is less than the available keccakfs, we need to fill the remaining inputs
        if num_inputs < self.num_available_keccakfs {
            // Compute the hash of zero
            let mut zero_output: [u64; 25] = [0u64; 25];
            keccakf(&mut zero_output);

            // If the number of inputs is not a multiple of NUM_KECCAKF_PER_SLOT,
            // we fill the last processed slot
            let rem_inputs = num_inputs % Self::NUM_KECCAKF_PER_SLOT;
            if num_inputs % Self::NUM_KECCAKF_PER_SLOT != 0 {
                let slot = (num_inputs - 1) / Self::NUM_KECCAKF_PER_SLOT;
                let slot_offset = slot * Self::BLOCKS_PER_SLOT;
                // Since no more bits are being introduced as input, we let 0 be the
                // new bits and therefore we repeat the last values
                for j in 0..RB * RB_BLOCKS_TO_PROCESS / 2 {
                    let block_offset = j * Self::NUM_KECCAKF_PER_SLOT;
                    for k in rem_inputs..Self::NUM_KECCAKF_PER_SLOT {
                        let pos = initial_offset + slot_offset + block_offset + k;
                        // trace[pos+1].bit1 = F::zero();
                        // trace[pos+1].bit2 = F::zero();
                        trace[pos + 1].val1 = trace[pos].val1;
                        trace[pos + 1].val2 = trace[pos].val2;
                    }
                }

                // Since the new bits are all zero, we have to set the hash of 0 as the respective output
                zero_output.iter().enumerate().for_each(|(j, &value)| {
                    let chunk_offset = j * Self::NUM_KECCAKF_PER_SLOT * 64 / 2;
                    for k in 0..64 {
                        let new_bit = (value >> k) & 1;
                        // In even bits, we update bit1 and val1; in odd bits, we update bit2 and val2
                        if k % 2 == 0 {
                            let bit_offset = k * Self::NUM_KECCAKF_PER_SLOT / 2;
                            for w in rem_inputs..Self::NUM_KECCAKF_PER_SLOT {
                                let pos = initial_offset
                                    + slot_offset
                                    + input_offset
                                    + chunk_offset
                                    + bit_offset
                                    + w;
                                trace[pos].bit1 = F::from_canonical_u64(new_bit);
                                trace[pos + 1].val1 = if w > 0 {
                                    // TODO: This check is not necessary
                                    trace[pos].val1 + F::from_canonical_u64(new_bit << w)
                                } else {
                                    F::from_canonical_u64(new_bit << w)
                                };
                            }
                        } else {
                            let bit_offset = (k - 1) * Self::NUM_KECCAKF_PER_SLOT / 2;
                            for w in rem_inputs..Self::NUM_KECCAKF_PER_SLOT {
                                let pos = initial_offset
                                    + slot_offset
                                    + input_offset
                                    + chunk_offset
                                    + bit_offset
                                    + w;
                                trace[pos].bit2 = F::from_canonical_u64(new_bit);
                                trace[pos + 1].val2 = if w > 0 {
                                    trace[pos].val2 + F::from_canonical_u64(new_bit << w)
                                } else {
                                    F::from_canonical_u64(new_bit << w)
                                };
                            }
                        }
                    }
                });
            }

            // Fill the remaining slots with the hash of 0
            let next_slot = num_inputs.div_ceil(Self::NUM_KECCAKF_PER_SLOT);
            zero_output.iter().enumerate().for_each(|(j, &value)| {
                for s in next_slot..self.num_available_slots {
                    let slot_offset = s * self.slot_size;
                    let chunk_offset = j * Self::NUM_KECCAKF_PER_SLOT * 64 / 2;
                    for k in 0..64 {
                        let new_bit = (value >> k) & 1;
                        // In even bits, we update bit1 and val1; in odd bits, we update bit2 and val2
                        if k % 2 == 0 {
                            let bit_offset = k * Self::NUM_KECCAKF_PER_SLOT / 2;
                            for w in 0..Self::NUM_KECCAKF_PER_SLOT {
                                let pos = initial_offset
                                    + slot_offset
                                    + input_offset
                                    + chunk_offset
                                    + bit_offset
                                    + w;
                                trace[pos].bit1 = F::from_canonical_u64(new_bit);
                                trace[pos + 1].val1 = if w > 0 {
                                    trace[pos].val1 + F::from_canonical_u64(new_bit << w)
                                } else {
                                    F::from_canonical_u64(new_bit << w)
                                };
                            }
                        } else {
                            let bit_offset = (k - 1) * Self::NUM_KECCAKF_PER_SLOT / 2;
                            for w in 0..Self::NUM_KECCAKF_PER_SLOT {
                                let pos = initial_offset
                                    + slot_offset
                                    + input_offset
                                    + chunk_offset
                                    + bit_offset
                                    + w;
                                trace[pos].bit2 = F::from_canonical_u64(new_bit);
                                trace[pos + 1].val2 = if w > 0 {
                                    trace[pos].val2 + F::from_canonical_u64(new_bit << w)
                                } else {
                                    F::from_canonical_u64(new_bit << w)
                                };
                            }
                        }
                    }
                }
            });
        }

        // Set the values of free_in_a, free_in_b, free_in_c using the script
        let script = self.script.clone();
        let mut offset = 0;
        for i in 0..self.num_available_slots {
            let mut bit_input_pos = [0u64; INPUT_DATA_SIZE_BITS];
            let mut bit_output_pos = [0u64; INPUT_DATA_SIZE_BITS];
            for j in 0..self.slot_size {
                let line = &script.program[j];
                let row = line.ref_ + i * self.slot_size;

                let a = &line.a;
                match a {
                    ValueType::Input(a) => {
                        set_col(trace, |row| &mut row.free_in_a, row, inputs_bits[i][a.bit]);
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
                        set_col(trace, |row| &mut row.free_in_b, row, inputs_bits[i][b.bit]);
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

                let a_val = get_col(trace, |row| &mut row.free_in_a, row) & MASK_CHUNK_BITS_KECCAKF;
                let b_val = get_col(trace, |row| &mut row.free_in_b, row) & MASK_CHUNK_BITS_KECCAKF;
                let op = &line.op;
                let c_val;
                if op == "xor" {
                    c_val = a_val ^ b_val;
                } else if op == "andp" {
                    c_val = (a_val ^ MASK_CHUNK_BITS_KECCAKF) & b_val
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
                    0u64 => KeccakfTableGateOp::Xor,
                    1u64 => KeccakfTableGateOp::Andp,
                    _ => panic!("Invalid gate operation"),
                };
                for j in 0..CHUNKS_KECCAKF {
                    let a_val = F::as_canonical_u64(&a[j]);
                    let b_val = F::as_canonical_u64(&b[j]);
                    let table_row = KeccakfTableSM::calculate_table_row(&gate_op_val, a_val, b_val);
                    multiplicity[table_row] += 1;
                }
            }

            // TOOD: Get the keccak-f output for debugging
            // println!("\nInput (C): {:?}", print_seq_format(&bit_input_pos));
            // println!("\nOuput (C): {:?}", print_seq_format(&bit_output_pos));

            // Move to the next slot
            offset += self.slot_size;
        }

        fn update_bit_val<F: PrimeField64>(
            fixed: &KeccakfFixed<F>,
            trace: &mut KeccakfTrace<F>,
            pos: usize,
            new_bit: u64,
            slot_pos: usize,
            is_bit1: bool,
        ) {
            if is_bit1 {
                trace[pos].bit1 = F::from_canonical_u64(new_bit);
                trace[pos + 1].val1 = if fixed[pos].latch_num_keccakf == F::zero() {
                    trace[pos].val1 + F::from_canonical_u64(new_bit << slot_pos)
                } else {
                    F::from_canonical_u64(new_bit << slot_pos)
                };
            } else {
                trace[pos].bit2 = F::from_canonical_u64(new_bit);
                trace[pos + 1].val2 = if fixed[pos].latch_num_keccakf == F::zero() {
                    trace[pos].val2 + F::from_canonical_u64(new_bit << slot_pos)
                } else {
                    F::from_canonical_u64(new_bit << slot_pos)
                };
            }
        }

        fn set_col<F: PrimeField64>(
            trace: &mut KeccakfTrace<F>,
            cols: impl Fn(&mut KeccakfTraceRow<F>) -> &mut [F; CHUNKS_KECCAKF],
            index: usize,
            value: u64,
        ) {
            let mut _value = value;
            let row = &mut trace[index];
            let cols = cols(row);
            for i in 0..CHUNKS_KECCAKF {
                cols[i] = F::from_canonical_u64(_value & MASK_BITS_KECCAKF);
                _value >>= BITS_KECCAKF;
            }
        }

        fn get_col<F: PrimeField64>(
            trace: &mut KeccakfTrace<F>,
            cols: impl Fn(&mut KeccakfTraceRow<F>) -> &mut [F; CHUNKS_KECCAKF],
            index: usize,
        ) -> u64 {
            let mut value = 0;
            let row = &mut trace[index];
            let cols = cols(row);
            for i in 0..CHUNKS_KECCAKF {
                let col_i_val = F::as_canonical_u64(&cols[i]);
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
        inputs: &[Vec<OperationKeccakData<u64>>],
    ) -> AirInstance<F> {
        // Get the fixed cols
        let airgroup_id = KeccakfTrace::<usize>::AIRGROUP_ID;
        let air_id = KeccakfTrace::<usize>::AIR_ID;
        let fixed = KeccakfFixed::from_slice(sctx.get_fixed_slice(airgroup_id, air_id));

        timer_start_trace!(KECCAKF_TRACE);
        let mut keccakf_trace = KeccakfTrace::new();
        let num_rows = keccakf_trace.num_rows();

        // Flatten the inputs
        let inputs: Vec<OperationKeccakData<u64>> = inputs.into_iter().flatten().cloned().collect();

        // Check that we can fit all the keccakfs in the trace
        let num_inputs: usize = inputs.len();
        let num_slots_needed = num_inputs.div_ceil(Self::NUM_KECCAKF_PER_SLOT);
        let num_rows_constants = 1; // Number of rows used for the constants
        let num_rows_needed = num_rows_constants + num_slots_needed * self.slot_size;

        // Sanity checks TODO: Put only in debug mode
        assert!(num_inputs <= self.num_available_keccakfs);
        assert!(num_slots_needed <= self.num_available_slots);
        assert!(num_rows_needed <= num_rows);

        info!(
            "{}: ··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        // Initialize the multiplicity table
        let mut multiplicity_keccakf_table = vec![0u64; KeccakfTableTrace::<F>::NUM_ROWS];

        // Set a = 0b00..00 and b = 0b11..11 at the first row
        // Set, e.g., the operation to be an XOR and set c = 0b11..11 = b = a ^ b
        let mut row: KeccakfTraceRow<F> = Default::default();
        let zeros = 0u64;
        let ones = MASK_BITS_KECCAKF;
        let gate_op = fixed[0].GATE_OP.as_canonical_u64();
        // Sanity check
        if gate_op != KeccakfTableGateOp::Xor as u64 {
            panic!("Invalid initial dummy gate operation");
        }
        for i in 0..CHUNKS_KECCAKF {
            row.free_in_a[i] = F::from_canonical_u64(zeros);
            row.free_in_b[i] = F::from_canonical_u64(ones);
            row.free_in_c[i] = F::from_canonical_u64(ones);
        }
        // Update the multiplicity table
        let table_row = KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, zeros, ones);
        multiplicity_keccakf_table[table_row] += CHUNKS_KECCAKF as u64;

        // Assign the single constant row
        keccakf_trace[0] = row;

        // Fill the rest of the trace
        self.process_slice(&fixed, &mut keccakf_trace, &inputs, &mut multiplicity_keccakf_table);
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);
        // A row with all zeros satisfies the constraints (since both XOR(0,0) and ANDP(0,0) are 0)
        let padding_row: KeccakfTraceRow<F> = Default::default();
        for i in (1 + self.slot_size * self.num_available_slots)..num_rows {
            let gate_op = fixed[i].GATE_OP.as_canonical_u64();
            // Sanity check
            if gate_op != KeccakfTableGateOp::Xor as u64 {
                panic!("Invalid initial dummy gate operation");
            }
            let table_row = KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, 0, 0);
            multiplicity_keccakf_table[table_row] += CHUNKS_KECCAKF as u64;

            keccakf_trace[i] = padding_row;
        }
        timer_stop_and_log_trace!(KECCAKF_PADDING);

        timer_start_trace!(KECCAKF_TABLE);
        self.keccakf_table_sm.process_slice(&multiplicity_keccakf_table);
        timer_stop_and_log_trace!(KECCAKF_TABLE);

        AirInstance::new_from_trace(FromTrace::new(&mut keccakf_trace))
    }

    /// Generates memory inputs.
    pub fn generate_inputs(input: &OperationKeccakData<u64>) -> Vec<Vec<PayloadType>> {
        // Get the basic data from the input
        let step = OperationBusData::get_a(&ExtOperationData::OperationKeccakData(*input));
        let addr = OperationBusData::get_b(&ExtOperationData::OperationKeccakData(*input));

        // Get the raw keccakf input as 25 u64 values
        let keccakf_input: [u64; 25] =
            OperationBusData::get_extra_data(&ExtOperationData::OperationKeccakData(*input))
                .try_into()
                .unwrap();

        // Apply the keccakf function and get the output
        let mut keccakf_output = keccakf_input.clone();
        keccakf(&mut keccakf_output);

        let mut mem_data = vec![];
        for i in 0..25 {
            mem_data.push(vec![step, addr, keccakf_input[i]]); // Read
            mem_data.push(vec![step, addr, keccakf_output[i]]); // Write
        }

        // mem_data
        // TODO: Finish!
        vec![]
    }
}
