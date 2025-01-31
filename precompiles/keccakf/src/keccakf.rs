use std::{fs, sync::Arc};

use log::info;
use p3_field::PrimeField64;

use data_bus::{
    ExtOperationData, OperationBusData, OperationData, OperationKeccakData, PayloadType,
};
use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{KeccakfTableTrace, KeccakfTrace, KeccakfTraceRow};

use crate::{keccakf_constants::*, KeccakfTableGateOp, KeccakfTableSM, Script, ValueType};

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM {
    /// Reference to the Keccakf Table State Machine.
    keccakf_table_sm: Arc<KeccakfTableSM>,

    /// Script for the Keccakf's circuit representation
    script: Arc<Script>,

    /// Size of a slot in the trace. It corresponds to the number of gates in the circuit.
    pub slot_size: usize,

    /// Number of available slots in the trace.
    num_available_slots: usize,

    /// Number of available keccakfs in the trace.
    pub num_available_keccakfs: usize,
}

type KeccakfInput = [u64; INPUT_DATA_SIZE_BITS];

impl KeccakfSM {
    const MY_NAME: &'static str = "Keccakf ";

    const NUM_KECCAKF_PER_SLOT: usize = CHUNKS_KECCAKF * BITS_KECCAKF;

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
        trace: &mut KeccakfTrace<F>,
        num_slots: usize,
        inputs: &[OperationKeccakData<u64>],
        multiplicity: &mut [u64],
    ) {
        // Process the inputs
        let zero_input: KeccakfInput = [0u64; INPUT_DATA_SIZE_BITS];
        let mut inputs_raw: Vec<KeccakfInput> = vec![zero_input; num_slots];
        inputs.iter().enumerate().for_each(|(i, input)| {
            // Get the raw keccakf input as 25 u64 values
            let keccakf_input: Vec<u64> =
                OperationBusData::get_extra_data(&ExtOperationData::OperationKeccakData(*input));

            // Process the raw data
            let slot = i / Self::NUM_KECCAKF_PER_SLOT;
            keccakf_input.iter().enumerate().for_each(|(j, &value)| {
                // Divide the value in bits
                for k in 0..64 {
                    let bit_pos = k + 64 * j;
                    let old_value = inputs_raw[slot][bit_pos];
                    let new_bit = (value >> k) & 1;
                    inputs_raw[slot][bit_pos] = (old_value << 1) | new_bit;
                }
            });

            // TODO: Compute the output of the keccakf??
            // TODO: Write the raw input/output to memory??
            // TODO: The previous memory calls should give me a_src_mem, c_src_mem

            // Get the basic data from the input
            let debug_main_step =
                OperationBusData::get_step(&ExtOperationData::OperationKeccakData(*input));
            let step_input =
                OperationBusData::get_a(&ExtOperationData::OperationKeccakData(*input));
            let addr_input =
                OperationBusData::get_b(&ExtOperationData::OperationKeccakData(*input));

            trace[i + 1] = KeccakfTraceRow {
                multiplicity: F::one(), // The pair (step_input, addr_input) is unique each time, so its multiplicity is 1
                debug_main_step: F::from_canonical_u64(debug_main_step),
                step_input: F::from_canonical_u64(step_input),
                addr_input: F::from_canonical_u64(addr_input),
                a_src_mem: F::zero(),
                c_src_mem: F::zero(),
                ..Default::default()
            };
        });

        // Set the remaining columns by using the script
        let script = self.script.clone();
        let mut offset = 0;
        for i in 0..num_slots {
            for j in 0..self.slot_size {
                let line = &script.program[j];
                let gate_ref = line.ref_ + i * self.slot_size;

                let a = &line.a;
                match a {
                    ValueType::Input(input_data) => {
                        set_col(
                            trace,
                            |row| &mut row.free_in_a,
                            gate_ref,
                            inputs_raw[i][input_data.bit],
                        );
                    }
                    ValueType::Wired(wired_data) => {
                        let mut gate = wired_data.gate;
                        if gate > 0 {
                            gate += offset;
                        }

                        let pin = &wired_data.pin;
                        if pin == "a" {
                            let pinned_value = get_col(trace, |row| &mut row.free_in_a, gate);
                            set_col(trace, |row| &mut row.free_in_a, gate_ref, pinned_value);
                        } else if pin == "b" {
                            let pinned_value = get_col(trace, |row| &mut row.free_in_b, gate);
                            set_col(trace, |row| &mut row.free_in_a, gate_ref, pinned_value);
                        } else if pin == "c" {
                            let pinned_value = get_col(trace, |row| &mut row.free_in_c, gate);
                            set_col(trace, |row| &mut row.free_in_a, gate_ref, pinned_value);
                        } else {
                            panic!("Invalid pin");
                        }
                    }
                }

                let b = &line.b;
                match b {
                    ValueType::Input(input_data) => {
                        set_col(
                            trace,
                            |row| &mut row.free_in_b,
                            gate_ref,
                            inputs_raw[i][input_data.bit],
                        );
                    }
                    ValueType::Wired(wired_data) => {
                        let mut gate = wired_data.gate;
                        if gate > 0 {
                            gate += offset;
                        }

                        let pin = &wired_data.pin;
                        if pin == "a" {
                            let pinned_value = get_col(trace, |row| &mut row.free_in_a, gate);
                            set_col(trace, |row| &mut row.free_in_b, gate_ref, pinned_value);
                        } else if pin == "b" {
                            let pinned_value = get_col(trace, |row| &mut row.free_in_b, gate);
                            set_col(trace, |row| &mut row.free_in_b, gate_ref, pinned_value);
                        } else if pin == "c" {
                            let pinned_value = get_col(trace, |row| &mut row.free_in_c, gate);
                            set_col(trace, |row| &mut row.free_in_b, gate_ref, pinned_value);
                        } else {
                            panic!("Invalid pin");
                        }
                    }
                }

                let a_val = get_col(trace, |row| &mut row.free_in_a, gate_ref);
                let b_val = get_col(trace, |row| &mut row.free_in_b, gate_ref);
                let op = &line.op;
                let table_op: KeccakfTableGateOp;
                if op == "xor" {
                    set_col(trace, |row| &mut row.free_in_c, gate_ref, a_val ^ b_val);
                    table_op = KeccakfTableGateOp::Xor;
                } else if op == "andp" {
                    set_col(
                        trace,
                        |row| &mut row.free_in_c,
                        gate_ref,
                        (a_val ^ MASK_CHUNK_BITS_KECCAKF) & b_val,
                    );
                    table_op = KeccakfTableGateOp::Andp;
                } else {
                    panic!("Invalid operation");
                }
            }

            // Update the multiplicity table for the slot
            for i in 0..self.slot_size {
                let a = trace[i].free_in_a;
                let b = trace[i].free_in_b;
                let table_op = KeccakfTableGateOp::Andp; // TODO: Consult the fixed column!
                for j in 0..CHUNKS_KECCAKF {
                    let a_val = F::as_canonical_u64(&a[j]);
                    let b_val = F::as_canonical_u64(&b[j]);
                    let table_row = KeccakfTableSM::calculate_table_row(&table_op, a_val, b_val);
                    multiplicity[table_row as usize] += 1;
                }
            }

            // Move to the next slot
            offset += self.slot_size;
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
            value & MASK_CHUNK_BITS_KECCAKF
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
        inputs: &[OperationKeccakData<u64>],
    ) -> AirInstance<F> {
        let mut keccakf_trace = KeccakfTrace::new();

        timer_start_trace!(KECCAKF_TRACE);
        let num_rows = keccakf_trace.num_rows();

        // Check that we can fit all the keccakfs in the trace
        let num_inputs = inputs.len();
        assert!(num_inputs <= self.num_available_keccakfs);

        let num_slots_needed = num_inputs.div_ceil(Self::NUM_KECCAKF_PER_SLOT);
        assert!(num_slots_needed <= self.num_available_slots); // Redundant, given the previous assert

        let num_rows_needed = 1 + num_slots_needed * self.slot_size;

        info!(
            "{}: ··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        // Initialize the multiplicity table
        let mut multiplicity_table = vec![0u64; KeccakfTableTrace::<F>::NUM_ROWS];

        // Fill the first row with 0b00..00 and 0b11..11
        let mut first_row: KeccakfTraceRow<F> = Default::default();
        let zeros = 0u64;
        let ones = MASK_BITS_KECCAKF;
        for i in 0..CHUNKS_KECCAKF {
            first_row.free_in_a[i] = F::from_canonical_u64(zeros);
            first_row.free_in_b[i] = F::from_canonical_u64(ones);
            // 0b00..00 ^ 0b11..11 = 0b11..11 (assuming GATE_OP refers to XOR)
            first_row.free_in_c[i] = F::from_canonical_u64(ones);
        }
        // Update the multiplicity table
        let table_row = KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, zeros, ones);
        multiplicity_table[table_row as usize] += CHUNKS_KECCAKF as u64;

        // Assign the first row
        keccakf_trace[0] = first_row;

        // Fill the rest of the trace
        self.process_slice(&mut keccakf_trace, num_slots_needed, inputs, &mut multiplicity_table);
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);
        // A row with all zeros satisfies the constraints (since both XOR(0,0) and ANDP(0,0) are 0)
        let padding_row: KeccakfTraceRow<F> = Default::default();
        for i in (1 + self.slot_size*num_slots_needed)..num_rows {
            keccakf_trace[i] = padding_row;

            let gete_op = KeccakfTableGateOp::Xor; // TODO: Consult the fixed column!
            let table_row = KeccakfTableSM::calculate_table_row(&gete_op, 0, 0);
            multiplicity_table[table_row as usize] += 1;
        }
        timer_stop_and_log_trace!(KECCAKF_PADDING);

        timer_start_trace!(KECCAKF_TABLE);
        self.keccakf_table_sm.process_slice(&multiplicity_table);
        timer_stop_and_log_trace!(KECCAKF_TABLE);

        AirInstance::new_from_trace(FromTrace::new(&mut keccakf_trace))
    }

    /// Generates memory inputs.
    pub fn generate_inputs(input: &OperationData<u64>) -> Vec<Vec<PayloadType>> {
        let debug_main_step =
            OperationBusData::get_step(&data_bus::ExtOperationData::OperationData(*input));
        let step_input =
            OperationBusData::get_a(&data_bus::ExtOperationData::OperationData(*input));
        let addr_input =
            OperationBusData::get_b(&data_bus::ExtOperationData::OperationData(*input));

        // TODO: Get the raw inputs from memory
        // TODO: Compute the output of the keccakf
        // TODO: Write the raw output to memory??

        if true {
            // TODO: We dont need to "glue" the d,b chunks back, we can use the aop API to do
            // vec![OperationBusData::from_values(
            //     step,
            //     opcode,
            //     ZiskOperationType::Binary as u64,
            //     aop.d[0] +
            //         CHUNK_SIZE * aop.d[1] +
            //         CHUNK_SIZE.pow(2) * (aop.d[2] + extension.0) +
            //         CHUNK_SIZE.pow(3) * aop.d[3],
            //     aop.b[0] +
            //         CHUNK_SIZE * aop.b[1] +
            //         CHUNK_SIZE.pow(2) * (aop.b[2] + extension.1) +
            //         CHUNK_SIZE.pow(3) * aop.b[3],
            // )
            // .to_vec()]
            vec![]
        } else {
            vec![]
        }
    }
}
