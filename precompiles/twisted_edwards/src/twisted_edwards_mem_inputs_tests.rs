//! Layout test for the mem inputs of the twisted-Edwards adds (`babyjubjub_add`, `ed25519_add`),
//! called with two direct operands: p1 (which receives the result) lives at `a`, p2 at `b`, and
//! there is no parameter struct, so no indirection is read.

use proofman_fields::Goldilocks;
use zisk_common::{
    ExtOperationData, A, B, OPERATION_BUS_BABYJUBJUB_ADD_DATA_SIZE,
    OPERATION_BUS_ED25519_ADD_DATA_SIZE, STEP,
};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::ZiskOperationType;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use crate::{executors, TwistedEdwardsAddInput, TwistedEdwardsCurve, TwistedEdwardsSM};

#[derive(Default)]
struct Recorder {
    rows: Vec<[u64; 7]>,
}

impl MemProcessor for Recorder {
    fn process_mem_data(&mut self, data: &[u64; 7]) {
        self.rows.push(*data);
    }
    fn skip_addr(&mut self, _addr: u32) -> bool {
        false
    }
    fn skip_addr_range(&mut self, _addr_from: u32, _addr_to: u32) -> bool {
        false
    }
}

fn check_add_mem_inputs(op: u8, curve: TwistedEdwardsCurve, data_size: usize) {
    let (a, b, step) = (0xA001_0000u64, 0xA002_0040u64, 77u64);
    let p1: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let p2: [u64; 8] = [11, 12, 13, 14, 15, 16, 17, 18];

    // Bus payload of a direct a/b op: [op, op_type, a, b, step, p1..., p2...]
    let mut data = vec![op as u64, ZiskOperationType::TwistedEdwards as u64, a, b, step];
    data.extend_from_slice(&p1);
    data.extend_from_slice(&p2);
    assert_eq!(data.len(), data_size);

    let mut p3 = [0u64; 8];
    executors::TwistedEdwards::calculate_add(curve, &p1, &p2, &mut p3);

    // Reads of p1 at a, reads of p2 at b, and the result written back at a.
    let mut expected = Recorder::default();
    for (i, value) in p1.iter().enumerate() {
        MemBusHelpers::mem_aligned_op(a as u32 + 8 * i as u32, step, *value, false, &mut expected);
    }
    for (i, value) in p2.iter().enumerate() {
        MemBusHelpers::mem_aligned_op(b as u32 + 8 * i as u32, step, *value, false, &mut expected);
    }
    for (i, value) in p3.iter().enumerate() {
        MemBusHelpers::mem_aligned_op(a as u32 + 8 * i as u32, step, *value, true, &mut expected);
    }

    let mut recorder = Recorder::default();
    <TwistedEdwardsSM<Goldilocks> as PrecompileMemInputs>::generate(
        data[B] as u32,
        data[STEP],
        &data,
        false,
        &mut recorder,
    );
    assert_eq!(recorder.rows, expected.rows);

    // The witness input decodes the addresses from a and b too.
    let ext: ExtOperationData<u64> = data.as_slice().try_into().unwrap();
    let values = match ext {
        ExtOperationData::OperationBabyJubJubAddData(values) => {
            assert_eq!(curve, TwistedEdwardsCurve::BabyJubJub);
            values
        }
        ExtOperationData::OperationEd25519AddData(values) => {
            assert_eq!(curve, TwistedEdwardsCurve::Ed25519);
            values
        }
        _ => panic!("unexpected bus payload variant"),
    };
    let input = TwistedEdwardsAddInput::from(&values);
    assert_eq!((input.p1_addr as u64, input.p2_addr as u64, input.step), (data[A], data[B], step));
    assert_eq!((input.p1, input.p2), (p1, p2));
}

#[test]
fn babyjubjub_add_mem_inputs_take_the_operands_from_a_and_b() {
    check_add_mem_inputs(
        ZiskOp::BABYJUBJUB_ADD,
        TwistedEdwardsCurve::BabyJubJub,
        OPERATION_BUS_BABYJUBJUB_ADD_DATA_SIZE,
    );
}

#[test]
fn ed25519_add_mem_inputs_take_the_operands_from_a_and_b() {
    check_add_mem_inputs(
        ZiskOp::ED25519_ADD,
        TwistedEdwardsCurve::Ed25519,
        OPERATION_BUS_ED25519_ADD_DATA_SIZE,
    );
}
