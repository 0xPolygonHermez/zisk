use std::{collections::HashMap, sync::Arc};

use crate::{BinaryExtensionTableOp, BinaryExtensionTableSM};
use log::info;
use num_bigint::BigInt;
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::*;

const MASK_32: u64 = 0xFFFFFFFF;
const MASK_64: u64 = 0xFFFFFFFFFFFFFFFF;

const SE_MASK_32: u64 = 0xFFFFFFFF00000000;
const SE_MASK_16: u64 = 0xFFFFFFFFFFFF0000;
const SE_MASK_8: u64 = 0xFFFFFFFFFFFFFF00;

const SIGN_32_BIT: u64 = 0x80000000;
const SIGN_BYTE: u64 = 0x80;

const LS_5_BITS: u64 = 0x1F;
const LS_6_BITS: u64 = 0x3F;

pub struct BinaryExtensionSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // STD
    std: Arc<Std<F>>,

    // Secondary State machines
    binary_extension_table_sm: Arc<BinaryExtensionTableSM<F>>,
}

impl<F: PrimeField> BinaryExtensionSM<F> {
    const MY_NAME: &'static str = "BinaryE ";

    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        std: Arc<Std<F>>,
        binary_extension_table_sm: Arc<BinaryExtensionTableSM<F>>,
        airgroup_id: usize,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let binary_extension_sm =
            Arc::new(Self { wcm: wcm.clone(), std: std.clone(), binary_extension_table_sm });

        wcm.register_component(binary_extension_sm.clone(), Some(airgroup_id), Some(air_ids));

        std.register_predecessor();

        binary_extension_sm
    }

    pub fn operations() -> Vec<u8> {
        vec![
            ZiskOp::Sll.code(),
            ZiskOp::Srl.code(),
            ZiskOp::Sra.code(),
            ZiskOp::SllW.code(),
            ZiskOp::SrlW.code(),
            ZiskOp::SraW.code(),
            ZiskOp::SignExtendB.code(),
            ZiskOp::SignExtendH.code(),
            ZiskOp::SignExtendW.code(),
        ]
    }

    fn opcode_is_shift(opcode: ZiskOp) -> bool {
        match opcode {
            ZiskOp::Sll |
            ZiskOp::Srl |
            ZiskOp::Sra |
            ZiskOp::SllW |
            ZiskOp::SrlW |
            ZiskOp::SraW => true,

            ZiskOp::SignExtendB | ZiskOp::SignExtendH | ZiskOp::SignExtendW => false,

            _ => panic!("BinaryExtensionSM::opcode_is_shift() got invalid opcode={:?}", opcode),
        }
    }

    fn opcode_is_shift_word(opcode: ZiskOp) -> bool {
        match opcode {
            ZiskOp::SllW | ZiskOp::SrlW | ZiskOp::SraW => true,

            ZiskOp::Sll |
            ZiskOp::Srl |
            ZiskOp::Sra |
            ZiskOp::SignExtendB |
            ZiskOp::SignExtendH |
            ZiskOp::SignExtendW => false,

            _ => panic!("BinaryExtensionSM::opcode_is_shift() got invalid opcode={:?}", opcode),
        }
    }

    pub fn process_slice(
        operation: &ZiskRequiredOperation,
        multiplicity: &mut [u64],
        range_check: &mut HashMap<u64, u64>,
    ) -> BinaryExtensionRow<F> {
        // Get the opcode
        let op = operation.opcode;

        // Get a ZiskOp from the code
        let opcode = ZiskOp::try_from_code(operation.opcode).expect("Invalid ZiskOp opcode");

        // Create an empty trace
        let mut row =
            BinaryExtensionRow::<F> { op: F::from_canonical_u8(op), ..Default::default() };

        // Set if the opcode is a shift operation
        let op_is_shift = Self::opcode_is_shift(opcode);
        row.op_is_shift = F::from_bool(op_is_shift);

        // Set if the opcode is a shift word operation
        let op_is_shift_word = Self::opcode_is_shift_word(opcode);

        // Detect if this is a sign extend operation
        let a = if op_is_shift { operation.a } else { operation.b };
        let b = if op_is_shift { operation.b } else { operation.a };

        // Split a in bytes and store them in in1
        let a_bytes: [u8; 8] = a.to_le_bytes();
        for (i, value) in a_bytes.iter().enumerate() {
            row.in1[i] = F::from_canonical_u8(*value);
        }

        // Store b low part into in2_low
        let in2_low: u64 = if op_is_shift { b & 0xFF } else { 0 };
        row.in2_low = F::from_canonical_u64(in2_low);

        // Store b lower bits when shifting, depending on operation size
        let b_low = if op_is_shift_word { b & LS_5_BITS } else { b & LS_6_BITS };

        // Store b into in2
        let in2_0: u64 = if op_is_shift { (b >> 8) & 0xFFFFFF } else { b & 0xFFFFFFFF };
        let in2_1: u64 = (b >> 32) & 0xFFFFFFFF;
        row.in2[0] = F::from_canonical_u64(in2_0);
        row.in2[1] = F::from_canonical_u64(in2_1);

        // Set main SM step
        row.main_step = F::from_canonical_u64(operation.step);

        // Calculate the trace output
        let mut t_out: [[u64; 2]; 8] = [[0; 2]; 8];

        // Calculate output based on opcode
        let binary_extension_table_op: BinaryExtensionTableOp;
        match opcode {
            ZiskOp::Sll => {
                binary_extension_table_op = BinaryExtensionTableOp::Sll;
                for j in 0..8 {
                    let bits_to_shift = b_low + 8 * j as u64;
                    let out =
                        if bits_to_shift < 64 { (a_bytes[j] as u64) << bits_to_shift } else { 0 };
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::Srl => {
                binary_extension_table_op = BinaryExtensionTableOp::Srl;
                for j in 0..8 {
                    let out = ((a_bytes[j] as u64) << (8 * j as u64)) >> b_low;
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::Sra => {
                binary_extension_table_op = BinaryExtensionTableOp::Sra;
                for j in 0..8 {
                    let mut out = ((a_bytes[j] as u64) << (8 * j as u64)) >> b_low;
                    if j == 7 {
                        // most significant bit of most significant byte define if negative or not
                        // if negative then add b bits one on the left
                        if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                            out |= MASK_64 << (64 - b_low);
                        }
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SllW => {
                binary_extension_table_op = BinaryExtensionTableOp::SllW;
                for j in 0..8 {
                    let mut out: u64;
                    if j >= 4 {
                        out = 0;
                    } else {
                        out = (((a_bytes[j] as u64) << b_low) + (8 * j as u64)) & MASK_32;
                        if (out & SIGN_32_BIT) != 0 {
                            out |= SE_MASK_32;
                        }
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SrlW => {
                binary_extension_table_op = BinaryExtensionTableOp::SrlW;
                for j in 0..8 {
                    let mut out: u64;
                    if j >= 4 {
                        out = 0;
                    } else {
                        out = (((a_bytes[j] as u64) << (8 * j as u64)) >> b_low) & MASK_32;
                        if (out & SIGN_32_BIT) != 0 {
                            out |= SE_MASK_32;
                        }
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SraW => {
                binary_extension_table_op = BinaryExtensionTableOp::SraW;
                for j in 0..8 {
                    let mut out: u64;
                    if j >= 4 {
                        out = 0;
                    } else {
                        out = ((a_bytes[j] as u64) << (8 * j as u64)) >> b_low;
                        if j == 3 && ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                            out |= MASK_64 << (32 - b_low);
                        }
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SignExtendB => {
                binary_extension_table_op = BinaryExtensionTableOp::SignExtendB;
                for j in 0..8 {
                    let out: u64;
                    if j == 0 {
                        if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                            out = (a_bytes[j] as u64) | SE_MASK_8;
                        } else {
                            out = a_bytes[j] as u64;
                        }
                    } else {
                        out = 0;
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SignExtendH => {
                binary_extension_table_op = BinaryExtensionTableOp::SignExtendH;
                for j in 0..8 {
                    let out: u64;
                    if j == 0 {
                        out = a_bytes[j] as u64;
                    } else if j == 1 {
                        if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                            out = (a_bytes[j] as u64) | SE_MASK_16;
                        } else {
                            out = a_bytes[j] as u64;
                        }
                    } else {
                        out = 0;
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SignExtendW => {
                binary_extension_table_op = BinaryExtensionTableOp::SignExtendW;
                for j in 0..4 {
                    let mut out = (a_bytes[j] as u64) << (8 * j as u64);
                    if j == 3 && ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                        out |= SE_MASK_32;
                    }

                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            _ => panic!(
                "BinaryExtensionSM::process_slice() found invalid opcode={}",
                operation.opcode
            ),
        }

        // Convert the trace output to field elements
        for j in 0..8 {
            row.out[j as usize][0] = F::from_canonical_u64(t_out[j as usize][0]);
            row.out[j as usize][1] = F::from_canonical_u64(t_out[j as usize][1]);
        }

        // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
        row.multiplicity = F::one();

        for (i, a_byte) in a_bytes.iter().enumerate() {
            let row = BinaryExtensionTableSM::<F>::calculate_table_row(
                binary_extension_table_op,
                i as u64,
                *a_byte as u64,
                in2_low,
            );
            multiplicity[row as usize] += 1;
        }

        // Store the range check
        if op_is_shift {
            *range_check.entry(in2_0).or_insert(0) += 1;
        }

        // Return successfully
        row
    }

    pub fn prove_instance(
        &self,
        operations: &[ZiskRequiredOperation],
        binary_e_trace: &mut BinaryExtensionTrace<F>,
    ) {
        timer_start_debug!(BINARY_EXTENSION_TRACE);
        let pctx = self.wcm.get_pctx();

        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS[0]);
        let air_binary_extension_table =
            pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_EXTENSION_TABLE_AIR_IDS[0]);
        assert!(operations.len() <= air.num_rows());

        info!(
            "{}: ··· Creating Binary extension instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            operations.len(),
            air.num_rows(),
            operations.len() as f64 / air.num_rows() as f64 * 100.0
        );

        let mut multiplicity_table = vec![0u64; air_binary_extension_table.num_rows()];
        let mut range_check: HashMap<u64, u64> = HashMap::new();

        for (i, operation) in operations.iter().enumerate() {
            let row = Self::process_slice(operation, &mut multiplicity_table, &mut range_check);
            binary_e_trace[i] = row;
        }
        timer_stop_and_log_debug!(BINARY_EXTENSION_TRACE);

        timer_start_debug!(BINARY_EXTENSION_PADDING);
        let padding_row =
            BinaryExtensionRow::<F> { op: F::from_canonical_u64(0x25), ..Default::default() };

        for i in operations.len()..air.num_rows() {
            binary_e_trace[i] = padding_row;
        }

        let padding_size = air.num_rows() - operations.len();
        for i in 0..8 {
            let multiplicity = padding_size as u64;
            let row = BinaryExtensionTableSM::<F>::calculate_table_row(
                BinaryExtensionTableOp::SignExtendW,
                i,
                0,
                0,
            );
            multiplicity_table[row as usize] += multiplicity;
        }
        timer_stop_and_log_debug!(BINARY_EXTENSION_PADDING);

        timer_start_debug!(BINARY_EXTENSION_TABLE);
        self.binary_extension_table_sm.process_slice(&multiplicity_table);
        timer_stop_and_log_debug!(BINARY_EXTENSION_TABLE);

        let range_id = self.std.get_range(BigInt::from(0), BigInt::from(0xFFFFFF), None);
        timer_start_debug!(BINARY_EXTENSION_RANGE);
        for (value, multiplicity) in &range_check {
            self.std.range_check(
                F::from_canonical_u64(*value),
                F::from_canonical_u64(*multiplicity),
                range_id,
            );
        }
        timer_stop_and_log_debug!(BINARY_EXTENSION_RANGE);
    }
}

impl<F: PrimeField> WitnessComponent<F> for BinaryExtensionSM<F> {}
