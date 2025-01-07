use std::sync::{Arc, Mutex};

use p3_field::Field;
use zisk_core::{P2_16, P2_17, P2_18, P2_19, P2_8, P2_9};
use zisk_pil::BinaryTableTrace;

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryBasicTableOp {
    Minu = 0x02,
    Min = 0x03,
    Maxu = 0x04,
    Max = 0x05,
    LtAbsNP = 0x06,
    LtAbsPN = 0x07,
    Ltu = 0x08,
    Lt = 0x09,
    Gt = 0x0a,
    Eq = 0x0b,
    Add = 0x0c,
    Sub = 0x0d,
    Leu = 0x0e,
    Le = 0x0f,
    And = 0x10,
    Or = 0x11,
    Xor = 0x12,
    Ext32 = 0x13,
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
        flags: u64,
    ) -> u64 {
        debug_assert!(a <= 0xFF);
        debug_assert!(b <= 0xFF);
        debug_assert!(cin <= 0x03);
        debug_assert!(last <= 0x01);
        debug_assert!(flags <= 0x0F);

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
        }
    }

    fn opcode_has_last(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max
            | BinaryBasicTableOp::LtAbsNP
            | BinaryBasicTableOp::LtAbsPN
            | BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Gt
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub
            | BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le
            | BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor => true,
            BinaryBasicTableOp::Ext32 => false,
        }
    }

    fn opcode_has_cin(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max
            | BinaryBasicTableOp::LtAbsNP
            | BinaryBasicTableOp::LtAbsPN
            | BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Gt
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub => true,

            BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le
            | BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Ext32 => false,
        }
    }

    fn opcode_result_is_a(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max => true,

            BinaryBasicTableOp::LtAbsNP
            | BinaryBasicTableOp::LtAbsPN
            | BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Gt
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub
            | BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le
            | BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Ext32 => false,
        }
    }

    fn offset_opcode(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu => 0,
            BinaryBasicTableOp::Min => P2_19,
            BinaryBasicTableOp::Maxu => 2 * P2_19,
            BinaryBasicTableOp::Max => 3 * P2_19,
            BinaryBasicTableOp::LtAbsNP => 4 * P2_19,
            BinaryBasicTableOp::LtAbsPN => 5 * P2_19,
            BinaryBasicTableOp::Ltu => 6 * P2_19,
            BinaryBasicTableOp::Lt => 6 * P2_19 + P2_18,
            BinaryBasicTableOp::Gt => 6 * P2_19 + 2 * P2_18,
            BinaryBasicTableOp::Eq => 6 * P2_19 + 3 * P2_18,
            BinaryBasicTableOp::Add => 6 * P2_19 + 4 * P2_18,
            BinaryBasicTableOp::Sub => 6 * P2_19 + 5 * P2_18,
            BinaryBasicTableOp::Leu => 6 * P2_19 + 6 * P2_18,
            BinaryBasicTableOp::Le => 6 * P2_19 + 6 * P2_18 + P2_17,
            BinaryBasicTableOp::And => 6 * P2_19 + 6 * P2_18 + 2 * P2_17,
            BinaryBasicTableOp::Or => 6 * P2_19 + 6 * P2_18 + 3 * P2_17,
            BinaryBasicTableOp::Xor => 6 * P2_19 + 6 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Ext32 => 6 * P2_19 + 6 * P2_18 + 5 * P2_17,
        }
    }
}
