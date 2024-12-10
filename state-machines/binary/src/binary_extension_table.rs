use std::sync::{Arc, Mutex};

use p3_field::Field;
use zisk_core::{zisk_ops::ZiskOp, P2_11, P2_19, P2_8};
use zisk_pil::BinaryExtensionTableTrace;

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryExtensionTableOp {
    Sll = 0x0d,
    Srl = 0x0e,
    Sra = 0x0f,
    SllW = 0x1d,
    SrlW = 0x1e,
    SraW = 0x1f,
    SignExtendB = 0x23,
    SignExtendH = 0x24,
    SignExtendW = 0x25,
}

pub struct BinaryExtensionTableSM {
    // Row multiplicity table
    pub multiplicity: Mutex<Vec<u64>>,
}

#[derive(Debug)]
pub enum ExtensionTableSMErr {
    InvalidOpcode,
}

impl BinaryExtensionTableSM {
    pub fn new<F: Field>() -> Arc<Self> {
        let binary_extension_table =
            Self { multiplicity: Mutex::new(vec![0; BinaryExtensionTableTrace::<F>::NUM_ROWS]) };

        Arc::new(binary_extension_table)
    }

    pub fn operations() -> Vec<u8> {
        // TODO! Review this codes
        vec![
            ZiskOp::Sll.code(),
            ZiskOp::Srl.code(),
            ZiskOp::Sra.code(),
            ZiskOp::SignExtendB.code(),
            ZiskOp::SignExtendH.code(),
            ZiskOp::SignExtendW.code(),
        ]
    }

    pub fn process_slice(&self, input: &[u64]) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val;
        }
    }

    //lookup_proves(BINARY_EXTENSION_TABLE_ID, [OP, OFFSET, A, B, C0, C1], multiplicity);
    pub fn calculate_table_row(opcode: BinaryExtensionTableOp, offset: u64, a: u64, b: u64) -> u64 {
        // Calculate the different row offset contributors, according to the PIL
        assert!(a <= 0xFF);
        let offset_a: u64 = a;
        assert!(offset < 0x08);
        let offset_offset: u64 = offset * P2_8;
        assert!(b <= 0xFF);
        let offset_b: u64 = b * P2_11;
        let offset_opcode: u64 = Self::offset_opcode(opcode);

        offset_a + offset_offset + offset_b + offset_opcode
        //assert!(row < self.num_rows as u64);
    }

    fn offset_opcode(opcode: BinaryExtensionTableOp) -> u64 {
        match opcode {
            BinaryExtensionTableOp::Sll => 0,
            BinaryExtensionTableOp::Srl => P2_19,
            BinaryExtensionTableOp::Sra => 2 * P2_19,
            BinaryExtensionTableOp::SllW => 3 * P2_19,
            BinaryExtensionTableOp::SrlW => 4 * P2_19,
            BinaryExtensionTableOp::SraW => 5 * P2_19,
            BinaryExtensionTableOp::SignExtendB => 6 * P2_19,
            BinaryExtensionTableOp::SignExtendH => 6 * P2_19 + P2_11,
            BinaryExtensionTableOp::SignExtendW => 6 * P2_19 + 2 * P2_11,
            //_ => panic!("BinaryExtensionTableSM::offset_opcode() got invalid opcode={:?}", opcode),
        }
    }
}
