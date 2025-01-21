
use std::{fs, sync::Arc};

// use crate::{binary_constants::*, BinaryBasicTableOp, BinaryBasicTableSM};
use log::info;
use p3_field::PrimeField;
use precompiles_common::{PrecompileCall, PrecompileCode};
use serde::Deserialize;
use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use std::cmp::Ordering as CmpOrdering;
// use zisk_common::{OperationBusData, OperationData};
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{KeccakfTableTrace, KeccakfTrace, KeccakfTraceRow};

pub const KECCAK_OPCODE: u16 = 0x010101;

// Parameters
const CHUNKS: u64 = 5;
const CHUNK_BITS: u64 = 12;
const MASK_CHUNKBITS: u64 = (1 << CHUNK_BITS) - 1;

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM {
    /// Reference to the Keccakf Table State Machine.
    keccakf_table_sm: Arc<KeccakfTableSM>,
}

#[derive(Deserialize, Debug)]
struct Script {
    xor: u32,
    andp: u32,
    #[serde(rename = "maxRef")]
    maxref: u32,
    program: Vec<ProgramEntry>,
}

#[derive(Deserialize, Debug)]
struct ProgramEntry {
    a: InputType,
    b: InputType,
    op: String,
    #[serde(rename = "ref")]
    ref_: u32,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum InputType {
    Bit(BitInfo),
    Wired(WiredInfo),
}

#[derive(Deserialize, Debug)]
struct BitInfo {
    bit: u32,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Deserialize, Debug)]
struct WiredInfo {
    gate: u32,
    pin: String,
    #[serde(rename = "type")]
    type_: String,
}

impl KeccakfSM {
    const MY_NAME: &'static str = "Keccakf ";

    // Keccakf circuit size
    const CIRCUIT_ANDPS: usize = 38400;
    const CIRCUIT_XORS: usize = 116886;
    const SLOT_SIZE: usize = CIRCUIT_ANDPS + CIRCUIT_XORS;

    pub fn new(keccakf_table_sm: Arc<KeccakfTableSM>) -> Arc<Self> {
        Arc::new(Self { keccakf_table_sm })
    }

    // fn execute(opcode: u8, a: u64, b: u64) -> (u64, bool) {
    //     let is_zisk_op = ZiskOp::try_from_code(opcode).is_ok();
    //     if is_zisk_op {
    //         ZiskOp::execute(opcode, a, b)
    //     } else {
    //         match opcode {
    //             LT_ABS_NP_OP => Self::lt_abs_np_execute(a, b),
    //             LT_ABS_PN_OP => Self::lt_abs_pn_execute(a, b),
    //             GT_OP => Self::gt_execute(a, b),
    //             _ => panic!("KeccakfSM::execute() got invalid opcode={:?}", opcode),
    //         }
    //     }
    // }

    #[inline(always)]
    pub fn process_slice<F: PrimeField>(
        input: &OperationData<u64>,
        multiplicity: &mut [u64],
    ) -> KeccakfTraceRow<F> {
        // Create an empty row
        let mut row: KeccakfTraceRow<F> = Default::default();

        // Read the keccakf script
        let script = fs::read_to_string("keccakf_script.json")?;
        let script: Script = serde_json::from_str(&script)?;
        assert!(script.program.len() == Self::SLOT_SIZE);

        // Execute the opcode
        let opcode = OperationBusData::get_op(input);
        let a = OperationBusData::get_a(input);
        let b = OperationBusData::get_b(input);
        let step = OperationBusData::get_step(input);

        let (c, _) = Self::execute(opcode, a, b);

        // Split a in bytes and store them in free_in_a
        let a_bytes: [u8; 8] = a.to_le_bytes();
        for (i, value) in a_bytes.iter().enumerate() {
            row.free_in_a[i] = F::from_canonical_u8(*value);
        }

        // Split b in bytes and store them in free_in_b
        let b_bytes: [u8; 8] = b.to_le_bytes();
        for (i, value) in b_bytes.iter().enumerate() {
            row.free_in_b[i] = F::from_canonical_u8(*value);
        }

        // Split c in bytes and store them in free_in_c
        let c_bytes: [u8; 8] = c.to_le_bytes();
        for (i, value) in c_bytes.iter().enumerate() {
            row.free_in_c[i] = F::from_canonical_u8(*value);
        }

        // Set main SM step
        row.debug_main_step = F::from_canonical_u64(step);

        let keccakf_table_op: BinaryBasicTableOp;
        match opcode {
            OR_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                keccakf_table_op = BinaryBasicTableOp::Or;

                row.use_last_carry = F::zero();

                // Set has initial carry
                row.has_initial_carry = F::zero();

                // No carry
                for i in 0..8 {
                    row.carry[i] = F::zero();

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = 0;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        keccakf_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        0,
                        plast[i],
                        flags,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            _ => panic!("KeccakfSM::process_slice() found invalid opcode={}", opcode),
        }

        // Set cout
        let cout32 = row.carry[HALF_BYTES - 1];
        let cout64 = row.carry[BYTES - 1];
        row.cout = mode64 * (cout64 - cout32) + cout32;

        // Set result_is_a
        row.result_is_a = row.op_is_min_max * row.cout;

        // Set use_last_carry_mode32 and use_last_carry_mode64
        row.use_last_carry_mode32 = F::from_bool(mode32) * row.use_last_carry;
        row.use_last_carry_mode64 = mode64 * row.use_last_carry;

        // Set micro opcode
        row.m_op = F::from_canonical_u8(keccakf_table_op as u8);

        // Set m_op_or_ext
        let ext_32_op = F::from_canonical_u8(BinaryBasicTableOp::Ext32 as u8);
        row.m_op_or_ext = mode64 * (row.m_op - ext_32_op) + ext_32_op;

        // Set free_in_a_or_c and free_in_b_or_zero
        for i in 0..HALF_BYTES {
            row.free_in_a_or_c[i] = mode64
                * (row.free_in_a[i + HALF_BYTES] - row.free_in_c[HALF_BYTES - 1])
                + row.free_in_c[HALF_BYTES - 1];
            row.free_in_b_or_zero[i] = mode64 * row.free_in_b[i + HALF_BYTES];
        }

        if row.use_last_carry == F::one() {
            // Set first and last elements
            row.free_in_c[7] = row.free_in_c[0];
            row.free_in_c[0] = F::zero();
        }

        // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
        row.multiplicity = F::one();

        // Return
        row
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `operations` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<F: PrimeField>(&self, inputs: &[OperationData<u64>]) -> AirInstance<F> {
        let mut keccakf_trace = KeccakfTrace::new();

        timer_start_trace!(KECCAKF_TRACE);
        let num_rows = keccakf_trace.num_rows();

        // TODO
        // assert!(inputs.len() <= num_rows);

        let num_slots = (num_rows - 1) / Self::SLOT_SIZE;

        // TODO!
        info!(
            "{}: ··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            inputs.len(),
            num_rows,
            inputs.len() as f64 / num_rows as f64 * 100.0
        );

        // Fill the first row with 0b00..00 and 0b11..11
        let mut first_row: KeccakfTraceRow<F> = Default::default();
        for i in 0..CHUNKS {
            first_row.free_in_a[i] = F::from_canonical_u64(0);
            first_row.free_in_b[i] = F::from_canonical_u64(MASK_CHUNKBITS);
            // 0b00..00 ^ 0b11..11 = 0b11..11 (assuming GATE_OP refers to XOR)
            first_row.free_in_c[i] = F::from_canonical_u64(MASK_CHUNKBITS);
        }
        keccakf_trace[0] = first_row;

        // Fill the rest of the trace
        let mut multiplicity_table = vec![0u64; KeccakfTableTrace::<F>::NUM_ROWS];
        for (i, operation) in inputs.iter().enumerate() {
            let row = Self::process_slice(operation, &mut multiplicity_table);
            keccakf_trace[i] = row;
        }
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);
        // A row with all zeros satisfies the constraints (assuming GATE_OP refers to XOR)
        let padding_row: KeccakfTraceRow<F> = Default::default();

        for i in inputs.len()..num_rows {
            keccakf_trace[i] = padding_row;
        }

        let padding_size = num_rows - inputs.len();
        for last in 0..2 {
            let multiplicity = (7 - 6 * last as u64) * padding_size as u64;
            let row = BinaryBasicTableSM::calculate_table_row(
                BinaryBasicTableOp::And,
                0,
                0,
                0,
                last as u64,
                0,
            );
            multiplicity_table[row as usize] += multiplicity;
        }
        timer_stop_and_log_trace!(KECCAKF_PADDING);

        timer_start_trace!(KECCAKF_TABLE);
        self.keccakf_table_sm.process_slice(&multiplicity_table);
        timer_stop_and_log_trace!(KECCAKF_TABLE);

        AirInstance::new_from_trace(FromTrace::new(&mut keccakf_trace))
    }
}

impl PrecompileCall for KeccakfSM {
    fn execute(&self, opcode: PrecompileCode, a: u64, b: u64) -> Option<(u64, bool)> {
        unimplemented!();
    }
}