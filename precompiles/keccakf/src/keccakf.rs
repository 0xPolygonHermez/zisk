use core::panic;
use std::sync::Arc;

use log::info;
use p3_field::PrimeField64;
use tiny_keccak::keccakf;

use circuit::{Gate, GateOperation, PinId};
use precompiles_common::MemBusHelpers;
use precompiles_helpers::keccakf_topology;
use proofman_common::{create_pool, AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::{ExtOperationData, OperationBusData, OperationKeccakData, PayloadType};
use zisk_pil::{KeccakfFixed, KeccakfTrace, KeccakfTraceRow};

use super::{keccakf_constants::*, KeccakfTableGateOp, KeccakfTableSM};

use rayon::prelude::*;

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM {
    /// Reference to the Keccakf Table State Machine.
    keccakf_table_sm: Arc<KeccakfTableSM>,

    /// The circuit description of the Keccakf
    program: Vec<u64>,
    gates: Vec<Gate>,

    /// Size of a slot in the trace. It corresponds to the number of gates in the circuit.
    slot_size: usize,

    /// Number of available slots in the trace.
    num_available_slots: usize,

    /// Number of available keccakfs in the trace.
    pub num_available_keccakfs: usize,
}

type KeccakfInput = [u64; INPUT_DATA_SIZE_BITS];

impl KeccakfSM {
    const MY_NAME: &'static str = "Keccakf ";

    pub const NUM_KECCAKF_PER_SLOT: usize = CHUNKS_KECCAKF * BITS_KECCAKF;

    const RB_SIZE: usize = Self::NUM_KECCAKF_PER_SLOT * RB;
    const BLOCKS_PER_SLOT: usize = Self::NUM_KECCAKF_PER_SLOT * RB * RB_BLOCKS_TO_PROCESS;

    /// Creates a new Keccakf State Machine instance.
    ///
    /// # Arguments
    /// * `keccakf_table_sm` - An `Arc`-wrapped reference to the Keccakf Table State Machine.
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new(keccakf_table_sm: Arc<KeccakfTableSM>) -> Arc<Self> {
        // Get the slot size
        let keccakf_top = keccakf_topology();
        let keccakf_program = keccakf_top.program;
        let keccakf_gates = keccakf_top.gates;
        let slot_size = keccakf_program.len();

        // Compute some useful values
        let num_available_slots = (KeccakfTrace::<usize>::NUM_ROWS - 1) / slot_size;
        let num_available_keccakfs = Self::NUM_KECCAKF_PER_SLOT * num_available_slots;

        Arc::new(Self {
            keccakf_table_sm,
            program: keccakf_program,
            gates: keccakf_gates,
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
        num_rows_constants: usize,
        inputs: &[OperationKeccakData<u64>],
        local_keccakf_table_sm: &KeccakfTableSM,
        core_id: usize,
        n_cores: usize,
    ) {
        let pool = create_pool(core_id, n_cores);
        pool.install(|| {
            let num_inputs = inputs.len();
            let mut inputs_bits: Vec<KeccakfInput> =
                vec![[0u64; INPUT_DATA_SIZE_BITS]; self.num_available_slots];

            // Process the inputs
            let initial_offset = num_rows_constants;
            let input_offset = Self::BLOCKS_PER_SLOT / BITS_IN_PARALLEL_KECCAKF; // Length of the input data
            inputs.iter().enumerate().for_each(|(i, input)| {
                let input_data = ExtOperationData::OperationKeccakData(*input);

                // Get the basic data from the input
                let step_received = OperationBusData::get_a(&input_data);
                let addr_received = OperationBusData::get_b(&input_data);

                // Get the raw keccakf input as 25 u64 values
                let keccakf_input: [u64; 25] =
                    OperationBusData::get_extra_data(&input_data).try_into().unwrap();

                let slot = i / Self::NUM_KECCAKF_PER_SLOT;
                let slot_pos = i % Self::NUM_KECCAKF_PER_SLOT;
                let slot_offset = slot * self.slot_size;

                // Update the multiplicity for the input
                let initial_pos = initial_offset + slot_offset + slot_pos;
                trace[initial_pos].multiplicity = F::ONE; // The pair (step_received, addr_received) is unique each time, so its multiplicity is 1

                // Process the keccakf input
                keccakf_input.iter().enumerate().for_each(|(j, &value)| {
                    let chunk_offset = j * Self::RB_SIZE;
                    let pos = initial_pos + chunk_offset;

                    // At the beginning of each 64-bit chunk, we set the step and address
                    trace[pos].step = F::from_u64(step_received);
                    trace[pos].addr = F::from_u64(addr_received + 8 * j as u64);
                    trace[pos].is_val = F::ONE;

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
                        let bit_pos = k % BITS_IN_PARALLEL_KECCAKF;
                        let bit_offset =
                            (k - bit_pos) * Self::NUM_KECCAKF_PER_SLOT / BITS_IN_PARALLEL_KECCAKF;
                        update_bit_val(fixed, trace, pos + bit_offset, new_bit, slot_pos, bit_pos);
                    }
                });

                // Apply the keccakf function and get the output
                let mut keccakf_output = keccakf_input;
                keccakf(&mut keccakf_output);

                // Process the output
                keccakf_output.iter().enumerate().for_each(|(j, &value)| {
                    let chunk_offset = j * Self::RB_SIZE;
                    let pos = initial_pos + input_offset + chunk_offset;

                    // At the beginning of each 64-bit chunk, we set the step and address
                    trace[pos].step = F::from_u64(step_received);
                    trace[pos].addr = F::from_u64(addr_received + 8 * j as u64);
                    trace[pos].is_val = F::ONE;

                    // Process the 64-bit chunk
                    for k in 0..64 {
                        // We update bit[i] and val[i]
                        let new_bit = (value >> k) & 1;
                        let bit_pos = k % BITS_IN_PARALLEL_KECCAKF;
                        let bit_offset =
                            (k - bit_pos) * Self::NUM_KECCAKF_PER_SLOT / BITS_IN_PARALLEL_KECCAKF;
                        update_bit_val(fixed, trace, pos + bit_offset, new_bit, slot_pos, bit_pos);
                    }
                });

                // At the end of the outputs, we set the next step and address for the constraints to be satisfied
                let final_pos =
                    initial_pos + input_offset + (keccakf_output.len() - 1) * Self::RB_SIZE;
                trace[final_pos + Self::RB_SIZE].step = trace[final_pos].step;
                trace[final_pos + Self::RB_SIZE].addr = trace[final_pos].addr
            });

            // It the number of inputs is less than the available keccakfs, we need to fill the remaining inputs
            if num_inputs < self.num_available_keccakfs {
                // Compute the hash of zero
                let mut zero_output: [u64; 25] = [0u64; 25];
                keccakf(&mut zero_output);

                // If the number of inputs is not a multiple of NUM_KECCAKF_PER_SLOT,
                // we fill the last processed slot
                let rem_inputs = num_inputs % Self::NUM_KECCAKF_PER_SLOT;
                if rem_inputs != 0 {
                    let last_slot = (num_inputs - 1) / Self::NUM_KECCAKF_PER_SLOT;
                    let slot_offset = last_slot * self.slot_size;
                    // Since no more bits are being introduced as input, we let 0 be the
                    // new bits and therefore we repeat the last values
                    for j in 0..RB * RB_BLOCKS_TO_PROCESS / 2 {
                        let block_offset = j * Self::NUM_KECCAKF_PER_SLOT;
                        for k in rem_inputs..Self::NUM_KECCAKF_PER_SLOT {
                            let pos = initial_offset + slot_offset + block_offset + k;
                            for l in 0..BITS_IN_PARALLEL_KECCAKF {
                                // trace[pos+1].bit[l] = F::ZERO;
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
                            let bit_pos = k % BITS_IN_PARALLEL_KECCAKF;
                            let bit_offset = (k - bit_pos) * Self::NUM_KECCAKF_PER_SLOT
                                / BITS_IN_PARALLEL_KECCAKF;
                            for w in rem_inputs..Self::NUM_KECCAKF_PER_SLOT {
                                update_bit_val(
                                    fixed,
                                    trace,
                                    pos + bit_offset + w,
                                    new_bit,
                                    w,
                                    bit_pos,
                                );
                            }
                        }
                    });
                }

                // Fill the remaining slots with the hash of 0
                let next_slot = num_inputs.div_ceil(Self::NUM_KECCAKF_PER_SLOT);
                zero_output.iter().enumerate().for_each(|(j, &value)| {
                    for s in next_slot..self.num_available_slots {
                        let slot_offset = s * self.slot_size;
                        let chunk_offset = j * Self::RB_SIZE;
                        for k in 0..64 {
                            let new_bit = (value >> k) & 1;
                            let bit_pos = k % BITS_IN_PARALLEL_KECCAKF;
                            let bit_offset = (k - bit_pos) * Self::NUM_KECCAKF_PER_SLOT
                                / BITS_IN_PARALLEL_KECCAKF;
                            let pos = initial_offset
                                + slot_offset
                                + input_offset
                                + chunk_offset
                                + bit_offset;
                            for w in 0..Self::NUM_KECCAKF_PER_SLOT {
                                update_bit_val(fixed, trace, pos + w, new_bit, w, bit_pos);
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
                // while !par_traces.is_empty() {
                let take = self.slot_size.min(trace_slice.len());
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
                    if (STATE_IN_REF_0
                        ..=STATE_IN_REF_0
                            + (STATE_IN_NUMBER - STATE_IN_GROUP_BY) * STATE_IN_REF_DISTANCE
                                / STATE_IN_GROUP_BY
                            + (STATE_IN_GROUP_BY - 1))
                        .contains(&ref_a)
                        && ((ref_a - STATE_IN_REF_0) % STATE_IN_REF_DISTANCE < STATE_IN_GROUP_BY)
                        && matches!(wired_a, PinId::A)
                    {
                        let s = ref_a - STATE_IN_REF_0;
                        let bit_a =
                            s / STATE_IN_REF_DISTANCE * STATE_IN_GROUP_BY + s % STATE_IN_GROUP_BY;
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
                    if (STATE_IN_REF_0
                        ..=STATE_IN_REF_0
                            + (STATE_IN_NUMBER - STATE_IN_GROUP_BY) * STATE_IN_REF_DISTANCE
                                / STATE_IN_GROUP_BY
                            + (STATE_IN_GROUP_BY - 1))
                        .contains(&ref_b)
                        && ((ref_b - STATE_IN_REF_0) % STATE_IN_REF_DISTANCE < STATE_IN_GROUP_BY)
                        && matches!(wired_b, PinId::A)
                    {
                        let s = ref_b - STATE_IN_REF_0;
                        let bit_b =
                            s / STATE_IN_REF_DISTANCE * STATE_IN_GROUP_BY + s % STATE_IN_GROUP_BY;
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

                // Update the multiplicity table for the slot
                for k in 0..self.slot_size {
                    let a = par_trace[k].free_in_a;
                    let b = par_trace[k].free_in_b;
                    let gate_op = fixed[k + 1 + i * self.slot_size].GATE_OP;
                    let gate_op_val = match F::as_canonical_u64(&gate_op) {
                        0u64 => KeccakfTableGateOp::Xor,
                        1u64 => KeccakfTableGateOp::Andp,
                        _ => panic!("Invalid gate operation"),
                    };
                    for j in 0..CHUNKS_KECCAKF {
                        let a_val = F::as_canonical_u64(&a[j]);
                        let b_val = F::as_canonical_u64(&b[j]);
                        let table_row =
                            KeccakfTableSM::calculate_table_row(&gate_op_val, a_val, b_val);
                        local_keccakf_table_sm.update_input(table_row, 1);
                    }
                }
            })
        });

        fn update_bit_val<F: PrimeField64>(
            fixed: &KeccakfFixed<F>,
            trace: &mut KeccakfTrace<F>,
            pos: usize,
            new_bit: u64,
            slot_pos: usize,
            bit_pos: usize,
        ) {
            trace[pos].bit[bit_pos] = F::from_u64(new_bit);
            trace[pos + 1].val[bit_pos] = if fixed[pos].latch_num_keccakf == F::ZERO {
                trace[pos].val[bit_pos] + F::from_u64(new_bit << slot_pos)
            } else {
                F::from_u64(new_bit << slot_pos)
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
        inputs: &[Vec<OperationKeccakData<u64>>],
        core_id: usize,
        n_cores: usize,
    ) -> AirInstance<F> {
        // Get the fixed cols
        let airgroup_id = KeccakfTrace::<usize>::AIRGROUP_ID;
        let air_id = KeccakfTrace::<usize>::AIR_ID;
        let fixed_pols = sctx.get_fixed(airgroup_id, air_id);
        let fixed = KeccakfFixed::from_vec(fixed_pols);

        let local_keccakf_table_sm = KeccakfTableSM::new::<F>();

        timer_start_trace!(KECCAKF_TRACE);
        let mut keccakf_trace = KeccakfTrace::new();
        let num_rows = keccakf_trace.num_rows();

        // Flatten the inputs
        let inputs: Vec<OperationKeccakData<u64>> = inputs.iter().flatten().cloned().collect();

        // Check that we can fit all the keccakfs in the trace
        let num_inputs: usize = inputs.len();
        let num_slots_needed = num_inputs.div_ceil(Self::NUM_KECCAKF_PER_SLOT);
        let num_rows_constants = 1; // Number of rows used for the constants
        let num_rows_needed = num_rows_constants + num_slots_needed * self.slot_size;

        // Sanity checks
        debug_assert!(
            num_inputs <= self.num_available_keccakfs,
            "Exceeded available Keccakfs inputs: requested {}, but only {} are available.",
            num_inputs,
            self.num_available_keccakfs
        );
        debug_assert!(num_slots_needed <= self.num_available_slots);
        debug_assert!(num_rows_needed <= num_rows);

        info!(
            "{}: ··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
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
        local_keccakf_table_sm.update_input(table_row, CHUNKS_KECCAKF as u64);

        // Assign the single constant row
        keccakf_trace[0] = row;

        // Fill the rest of the trace
        self.process_slice(
            &fixed,
            &mut keccakf_trace,
            num_rows_constants,
            &inputs,
            &local_keccakf_table_sm,
            core_id,
            n_cores,
        );
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);
        // A row with all zeros satisfies the constraints (since both XOR(0,0) and ANDP(0,0) are 0)
        let padding_row: KeccakfTraceRow<F> = Default::default();
        for i in (num_rows_constants + self.slot_size * self.num_available_slots)..num_rows {
            let gate_op = fixed[i].GATE_OP.as_canonical_u64();
            // Sanity check
            debug_assert_eq!(
                gate_op,
                KeccakfTableGateOp::Xor as u64,
                "Invalid padding dummy gate operation"
            );

            let table_row = KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, 0, 0);
            local_keccakf_table_sm.update_input(table_row, CHUNKS_KECCAKF as u64);

            keccakf_trace[i] = padding_row;
        }
        timer_stop_and_log_trace!(KECCAKF_PADDING);

        self.keccakf_table_sm.acc_local_multiplicity(&local_keccakf_table_sm);

        AirInstance::new_from_trace(FromTrace::new(&mut keccakf_trace))
    }

    /// Generates memory inputs.
    pub fn generate_inputs(
        input: &OperationKeccakData<u64>,
        counters_mode: bool,
    ) -> Vec<Vec<PayloadType>> {
        // Get the basic data from the input
        let input_data = ExtOperationData::OperationKeccakData(*input);

        let step_main = OperationBusData::get_a(&input_data);
        let addr = OperationBusData::get_b(&input_data) as u32;

        let mut mem_data = vec![];
        if counters_mode {
            // On counter phase we don't need final values, we only need the
            // address and step
            // Compute the reads
            for i in 0..25 {
                let new_addr = addr + 8 * i as u32;
                let read = MemBusHelpers::mem_aligned_load(new_addr, step_main, 0);
                mem_data.push(read.to_vec());
            }

            // Compute the writes
            for i in 0..25 {
                let new_addr = addr + 8 * i as u32;
                let write = MemBusHelpers::mem_aligned_write(new_addr, step_main, 0);
                mem_data.push(write.to_vec());
            }

            return mem_data;
        }
        // Get the raw keccakf input as 25 u64 values
        let keccakf_input: [u64; 25] =
            OperationBusData::get_extra_data(&input_data).try_into().unwrap();

        // Apply the keccakf function and get the output
        let mut keccakf_output = keccakf_input;
        keccakf(&mut keccakf_output);

        // Compute the reads
        for (i, &input) in keccakf_input.iter().enumerate() {
            let new_addr = addr + 8 * i as u32;
            let read = MemBusHelpers::mem_aligned_load(new_addr, step_main, input);
            mem_data.push(read.to_vec());
        }

        // Compute the writes
        for (i, &output) in keccakf_output.iter().enumerate() {
            let new_addr = addr + 8 * i as u32;
            let write = MemBusHelpers::mem_aligned_write(new_addr, step_main, output);
            mem_data.push(write.to_vec());
        }

        mem_data
    }
}
