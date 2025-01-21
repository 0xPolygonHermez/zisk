use std::{fs, sync::Arc};

use log::info;
use p3_field::PrimeField;
use precompiles_common::{PrecompileCall, PrecompileCode};
use serde::Deserialize;

use data_bus::{OperationBusData, OperationData};
use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{KeccakfTableTrace, KeccakfTrace, KeccakfTraceRow};

use crate::{keccakf_constants::*, KeccakfTableGateOp, KeccakfTableSM};

pub const KECCAK_OPCODE: u16 = 0x010101;

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM {
    /// Reference to the Keccakf Table State Machine.
    keccakf_table_sm: Arc<KeccakfTableSM>,

    /// Script for the Keccakf's circuit representation
    script: Arc<Script>,

    /// Size of a slot in the trace. It corresponds to the number of gates in the circuit.
    slot_size: usize,
}

#[derive(Deserialize, Debug)]
struct Script {
    xor: usize,
    andp: usize,
    #[serde(rename = "maxRef")]
    maxref: usize,
    program: Vec<ProgramLine>,
}

#[derive(Deserialize, Debug)]
struct ProgramLine {
    a: ValueType,
    b: ValueType,
    op: String,
    #[serde(rename = "ref")]
    ref_: usize,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ValueType {
    Input(InputData),
    Wired(WiredData),
}

#[derive(Deserialize, Debug)]
struct InputData {
    bit: u32,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Deserialize, Debug)]
struct WiredData {
    gate: u32,
    pin: String,
    #[serde(rename = "type")]
    type_: String,
}

impl KeccakfSM {
    const MY_NAME: &'static str = "Keccakf ";

    /// Creates a new Keccakf State Machine instance.
    ///
    /// # Arguments
    /// * `keccakf_table_sm` - An `Arc`-wrapped reference to the Keccakf Table State Machine.
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new(keccakf_table_sm: Arc<KeccakfTableSM>) -> Arc<Self> {
        // Parse the script
        let script = fs::read_to_string("keccakf_script.json").unwrap();
        let script: Script = serde_json::from_str(&script).unwrap();
        let slot_size = script.maxref;

        assert!(script.xor + script.andp == slot_size);
        assert!(script.program.len() == slot_size);

        Arc::new(Self { keccakf_table_sm, script: Arc::new(script), slot_size })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Keccakf trace.
    /// * `num_slots` - The number of slots to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_slice<F: PrimeField>(
        &self,
        trace: &mut KeccakfTrace<F>,
        num_slots: usize,
        input: &OperationData<u64>,
        multiplicity: &mut [u64],
    ) {
        // Get the basic data from the input
        let debug_main_step = OperationBusData::get_step(input);
        let step_input = OperationBusData::get_a(input);
        let addr_input = OperationBusData::get_b(input);

        // TODO: Get the raw inputs from memory
        // TODO: Compute the output of the keccakf
        // TODO: Write the raw output to memory??

        // TODO: Collect the inputs as an array of num_slots elements, each of size CHUNKS * BITS
        let input = vec![0u64; CHUNKS * BITS];

        // Read the keccakf script
        let script = self.script.clone();

        // Create an empty row
        let mut row: KeccakfTraceRow<F> = Default::default();

        let mut offset = 0;
        // for i in 0..num_slots {
        //     for j in 0..self.slot_size {
        //         let line = &script.program[j];
        //         let gate_ref = line.ref_ + i * self.slot_size;

        //         let a = line.a;
        //         match a {
        //             ValueType::Input(input_data) => {
        //                 set_col(&mut row.free_in_a, gate_ref, input[i][input_data.bit]);
        //             }
        //             ValueType::Wired(wired_data) => {
        //                 let g = wired_data.gate;
        //                 if g > 0 {
        //                     g += offset;
        //                 }

        //                 match wired_data.pin {
        //                     String::from("a") => {
        //                         set_col(&mut row.free_in_a, gate_ref, get_col(row.free_in_a, g))
        //                     }
        //                     String::from("b") => {
        //                         set_col(&mut row.free_in_a, gate_ref, get_col(row.free_in_b, g))
        //                     }
        //                     String::from("c") => {
        //                         set_col(&mut row.free_in_a, gate_ref, get_col(row.free_in_c, g))
        //                     }
        //                     _ => panic!("Invalid pin"),
        //                 }
        //             }
        //         }

        //         let b = line.b;
        //         match b {
        //             ValueType::Input(input_data) => {
        //                 set_col(&mut row.free_in_b, gate_ref, input[i][input_data.bit]);
        //             }
        //             ValueType::Wired(wired_data) => {
        //                 let g = wired_data.gate;
        //                 if g > 0 {
        //                     g += offset;
        //                 }

        //                 match wired_data.pin {
        //                     String::from("a") => {
        //                         set_col(&mut row.free_in_b, gate_ref, get_col(row.free_in_a, g))
        //                     }
        //                     String::from("b") => {
        //                         set_col(&mut row.free_in_b, gate_ref, get_col(row.free_in_b, g))
        //                     }
        //                     String::from("c") => {
        //                         set_col(&mut row.free_in_b, gate_ref, get_col(row.free_in_c, g))
        //                     }
        //                     _ => panic!("Invalid pin"),
        //                 }
        //             }
        //         }

        //         let op = line.op;
        //         match op {
        //             String::from("xor") => set_col(
        //                 &mut row.free_in_c,
        //                 gate_ref,
        //                 get_col(row.free_in_a, gate_ref) ^ get_col(row.free_in_b, gate_ref),
        //             ),
        //             String::from("andp") => set_col(
        //                 &mut row.free_in_c,
        //                 gate_ref,
        //                 (get_col(row.free_in_a, gate_ref) ^ MASK_CHUNK_BITS) &
        //                     (get_col(row.free_in_b, gate_ref)),
        //             ),
        //             _ => panic!("Invalid operation"),
        //         }
        //     }

        //     offset += self.slot_size;
        // }

        // Set main SM step
        row.debug_main_step = F::from_canonical_u64(debug_main_step);
        row.step_input = F::from_canonical_u64(step_input);
        row.addr_input = F::from_canonical_u64(addr_input);

        // TODO
        row.multiplicity = F::one();

        // fn set_col<F: PrimeField>(col: &mut [F; CHUNKS], index: usize, value: F) {
        //     for i in 0..CHUNKS {
        //         col[i][index] = value & MASK_BITS;
        //         value >>= BITS;
        //     }
        // }

        // fn get_col(col: &[F; CHUNKS], index: usize) -> F {
        //     let mut value = F::zero();
        //     for i in 0..CHUNKS {
        //         value += col[i][index] << (i * BITS);
        //     }
        //     value & MASK_CHUNK_BITS
        // }
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<F: PrimeField>(&self, inputs: &[OperationData<u64>]) -> AirInstance<F> {
        let mut keccakf_trace = KeccakfTrace::new();

        timer_start_trace!(KECCAKF_TRACE);
        let num_rows = keccakf_trace.num_rows();

        let num_keccakf_per_slot = CHUNKS * BITS;
        let num_slots = (num_rows - 1) / self.slot_size;
        let num_keccakfs = num_keccakf_per_slot * num_slots;

        // Check that we can fit all the keccakfs in the trace
        let num_inputs = inputs.len();
        assert!(num_inputs <= num_keccakfs);

        let num_rows_needed = 1 + num_inputs.div_ceil(num_keccakf_per_slot) * self.slot_size;

        info!(
            "{}: ··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        // Fill the first row with 0b00..00 and 0b11..11
        let mut first_row: KeccakfTraceRow<F> = Default::default();
        for i in 0..CHUNKS {
            first_row.free_in_a[i] = F::from_canonical_u64(0);
            first_row.free_in_b[i] = F::from_canonical_u64(MASK_BITS);
            // 0b00..00 ^ 0b11..11 = 0b11..11 (assuming GATE_OP refers to XOR)
            first_row.free_in_c[i] = F::from_canonical_u64(MASK_BITS);
        }
        keccakf_trace[0] = first_row;

        // Fill the rest of the trace
        let mut multiplicity_table = vec![0u64; KeccakfTableTrace::<F>::NUM_ROWS];
        for operation in inputs.iter() {
            self.process_slice(
                &mut keccakf_trace,
                num_slots,
                operation,
                &mut multiplicity_table,
            );
        }
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);
        // A row with all zeros satisfies the constraints (assuming GATE_OP refers to XOR)
        let padding_row: KeccakfTraceRow<F> = Default::default();

        for i in inputs.len()..num_rows {
            keccakf_trace[i + 1] = padding_row;
        }

        let row = KeccakfTableSM::calculate_table_row(KeccakfTableGateOp::Xor, 0, 0);
        let padding_size = num_rows - inputs.len() - 1;
        let multiplicity = padding_size as u64;
        multiplicity_table[row as usize] += multiplicity;

        timer_stop_and_log_trace!(KECCAKF_PADDING);

        timer_start_trace!(KECCAKF_TABLE);
        self.keccakf_table_sm.process_slice(&multiplicity_table);
        timer_stop_and_log_trace!(KECCAKF_TABLE);

        AirInstance::new_from_trace(FromTrace::new(&mut keccakf_trace))
    }

    // Generates memory inputs.
    // pub fn generate_inputs(input: &OperationData<u64>) -> Vec<Vec<PayloadType>> {
    //     let mut aop = ArithOperation::new();
    //     let opcode = OperationBusData::get_op(input);
    //     let a = OperationBusData::get_a(input);
    //     let b = OperationBusData::get_b(input);
    //     let step = OperationBusData::get_step(input);

    //     aop.calculate(opcode, a, b);

    //     // If the operation is a division, then use the binary component
    //     // to check that the remainer is lower than the divisor
    //     if aop.div && !aop.div_by_zero {
    //         let opcode = match (aop.nr, aop.nb) {
    //             (false, false) => LTU_OP,
    //             (false, true) => LT_ABS_PN_OP,
    //             (true, false) => LT_ABS_NP_OP,
    //             (true, true) => GT_OP,
    //         };

    //         let extension = match (aop.m32, aop.nr, aop.nb) {
    //             (false, _, _) => (0, 0),
    //             (true, false, false) => (0, 0),
    //             (true, false, true) => (0, EXTENSION),
    //             (true, true, false) => (EXTENSION, 0),
    //             (true, true, true) => (EXTENSION, EXTENSION),
    //         };

    //         // TODO: We dont need to "glue" the d,b chunks back, we can use the aop API to do
    // this!         vec![OperationBusData::from_values(
    //             step,
    //             opcode,
    //             ZiskOperationType::Binary as u64,
    //             aop.d[0] +
    //                 CHUNK_SIZE * aop.d[1] +
    //                 CHUNK_SIZE.pow(2) * (aop.d[2] + extension.0) +
    //                 CHUNK_SIZE.pow(3) * aop.d[3],
    //             aop.b[0] +
    //                 CHUNK_SIZE * aop.b[1] +
    //                 CHUNK_SIZE.pow(2) * (aop.b[2] + extension.1) +
    //                 CHUNK_SIZE.pow(3) * aop.b[3],
    //         )
    //         .to_vec()]
    //     } else {
    //         vec![]
    //     }
    // }
}

impl PrecompileCall for KeccakfSM {
    fn execute(&self, opcode: PrecompileCode, a: u64, b: u64) -> Option<(u64, bool)> {
        unimplemented!();
    }
}