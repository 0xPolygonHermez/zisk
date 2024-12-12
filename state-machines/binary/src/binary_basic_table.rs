use std::sync::{Arc, Mutex};

use p3_field::Field;
use zisk_core::{P2_16, P2_17, P2_18, P2_19, P2_8, P2_9};
use zisk_pil::BinaryTableTrace;

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryBasicTableOp {
    Add = 0x02,
    Sub = 0x03,
    Ltu = 0x04,
    Lt = 0x05,
    Leu = 0x06,
    Le = 0x07,
    Eq = 0x08,
    Minu = 0x09,
    Min = 0x0a,
    Maxu = 0x0b,
    Max = 0x0c,
    And = 0x20,
    Or = 0x21,
    Xor = 0x22,
    Ext32 = 0x23,
}

pub struct BinaryBasicTableSM {
    // Row multiplicity table
    multiplicity: Mutex<Vec<u64>>,
}

impl BinaryBasicTableSM {
    pub fn new<F: Field>() -> Arc<Self> {
        Arc::new(Self { multiplicity: Mutex::new(vec![0; BinaryTableTrace::<F>::NUM_ROWS]) })
    }

    pub fn process_slice(&self, input: &[u64]) {
        // Create the trace vector
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val;
        }
    }

    pub fn detach_multiplicity(&self) -> Vec<u64> {
        let mut multiplicity = self.multiplicity.lock().unwrap();
        std::mem::take(&mut *multiplicity)
    }

    //lookup_proves(BINARY_TABLE_ID, [LAST, OP, A, B, CIN, C, FLAGS], multiplicity);
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_table_row(
        opcode: BinaryBasicTableOp,
        a: u64,
        b: u64,
        cin: u64,
        last: u64,
        _c: u64,
        flags: u64,
        _i: u64,
    ) -> u64 {
        // Calculate the different row offset contributors, according to the PIL
        if opcode == BinaryBasicTableOp::Ext32 {
            let offset_a: u64 = a;
            let offset_cin: u64 = cin * P2_8;
            let offset_result_is_a: u64 = match flags {
                0 => 0,
                2 => P2_9,
                6 => 3 * P2_9,
                _ => {
                    panic!("BinaryBasicTableSM::calculate_table_row() unexpected flags={}", flags)
                }
            };
            let offset_opcode: u64 = Self::offset_opcode(opcode);

            offset_a + offset_cin + offset_result_is_a + offset_opcode
        } else {
            let offset_a: u64 = a;
            let offset_b: u64 = b * P2_8;
            let offset_last: u64 = if Self::opcode_has_last(opcode) { last * P2_16 } else { 0 };
            let offset_cin: u64 = if Self::opcode_has_cin(opcode) { cin * P2_17 } else { 0 };
            let offset_result_is_a: u64 =
                if Self::opcode_result_is_a(opcode) && ((flags & 0x04) != 0) { P2_18 } else { 0 };
            let offset_opcode: u64 = Self::offset_opcode(opcode);

            offset_a + offset_b + offset_last + offset_cin + offset_result_is_a + offset_opcode
            //assert!(row < self.num_rows as u64);
        }
    }

    fn opcode_has_last(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Add |
            BinaryBasicTableOp::Sub |
            BinaryBasicTableOp::Ltu |
            BinaryBasicTableOp::Lt |
            BinaryBasicTableOp::Leu |
            BinaryBasicTableOp::Le |
            BinaryBasicTableOp::Eq |
            BinaryBasicTableOp::Minu |
            BinaryBasicTableOp::Min |
            BinaryBasicTableOp::Maxu |
            BinaryBasicTableOp::Max |
            BinaryBasicTableOp::And |
            BinaryBasicTableOp::Or |
            BinaryBasicTableOp::Xor => true,
            BinaryBasicTableOp::Ext32 => false,
            //_ => panic!("BinaryBasicTableSM::opcode_has_last() got invalid opcode={:?}", opcode),
        }
    }

    fn opcode_has_cin(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Add |
            BinaryBasicTableOp::Sub |
            BinaryBasicTableOp::Ltu |
            BinaryBasicTableOp::Lt |
            BinaryBasicTableOp::Eq |
            BinaryBasicTableOp::Minu |
            BinaryBasicTableOp::Min |
            BinaryBasicTableOp::Maxu |
            BinaryBasicTableOp::Max => true,

            BinaryBasicTableOp::Leu |
            BinaryBasicTableOp::Le |
            BinaryBasicTableOp::And |
            BinaryBasicTableOp::Or |
            BinaryBasicTableOp::Xor |
            BinaryBasicTableOp::Ext32 => false,
            //_ => panic!("BinaryBasicTableSM::opcode_has_cin() got invalid opcode={:?}", opcode),
        }
    }

    fn opcode_result_is_a(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max => true,

            BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub
            | BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Ext32 => false,
            //_ => panic!("BinaryBasicTableSM::opcode_result_is_a() got invalid opcode={:?}", opcode),
        }
    }

    fn offset_opcode(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu => 0,
            BinaryBasicTableOp::Min => P2_19,
            BinaryBasicTableOp::Maxu => 2 * P2_19,
            BinaryBasicTableOp::Max => 3 * P2_19,
            BinaryBasicTableOp::Ltu => 4 * P2_19,
            BinaryBasicTableOp::Lt => 4 * P2_19 + P2_18,
            BinaryBasicTableOp::Eq => 4 * P2_19 + 2 * P2_18,
            BinaryBasicTableOp::Add => 4 * P2_19 + 3 * P2_18,
            BinaryBasicTableOp::Sub => 4 * P2_19 + 4 * P2_18,
            BinaryBasicTableOp::Leu => 4 * P2_19 + 5 * P2_18,
            BinaryBasicTableOp::Le => 4 * P2_19 + 5 * P2_18 + P2_17,
            BinaryBasicTableOp::And => 4 * P2_19 + 5 * P2_18 + 2 * P2_17,
            BinaryBasicTableOp::Or => 4 * P2_19 + 5 * P2_18 + 3 * P2_17,
            BinaryBasicTableOp::Xor => 4 * P2_19 + 5 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Ext32 => 4 * P2_19 + 5 * P2_18 + 5 * P2_17,
            //_ => panic!("BinaryBasicTableSM::offset_opcode() got invalid opcode={:?}", opcode),
        }
    }
}
