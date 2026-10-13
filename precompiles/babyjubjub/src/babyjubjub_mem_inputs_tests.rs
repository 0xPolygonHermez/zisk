//! Layout test for the mem inputs of `babyjubjub_add`, called with two direct operands: p1 (which
//! receives the result) lives at `a`, p2 at `b`, and there is no parameter struct, so no
//! indirection is read.

use proofman_fields::Goldilocks;
use zisk_common::{ExtOperationData, A, B, OPERATION_BUS_BABYJUBJUB_ADD_DATA_SIZE, STEP};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::ZiskOperationType;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use crate::{executors, BabyJubJubAddInput, BabyJubJubSM};

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

#[test]
fn babyjubjub_add_mem_inputs_take_the_operands_from_a_and_b() {
    let (a, b, step) = (0xA001_0000u64, 0xA002_0040u64, 77u64);
    let p1: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let p2: [u64; 8] = [11, 12, 13, 14, 15, 16, 17, 18];

    // Bus payload of a direct a/b op: [op, op_type, a, b, step, p1..., p2...]
    let mut data =
        vec![ZiskOp::BABYJUBJUB_ADD as u64, ZiskOperationType::BabyJubJub as u64, a, b, step];
    data.extend_from_slice(&p1);
    data.extend_from_slice(&p2);
    assert_eq!(data.len(), OPERATION_BUS_BABYJUBJUB_ADD_DATA_SIZE);

    let mut p3 = [0u64; 8];
    executors::BabyJubJub::calculate_add(&p1, &p2, &mut p3);

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
    <BabyJubJubSM<Goldilocks> as PrecompileMemInputs>::generate(
        data[B] as u32,
        data[STEP],
        &data,
        false,
        &mut recorder,
    );
    assert_eq!(recorder.rows, expected.rows);

    // The witness input decodes the addresses from a and b too.
    let ext: ExtOperationData<u64> = data.as_slice().try_into().unwrap();
    let ExtOperationData::OperationBabyJubJubAddData(values) = ext else {
        panic!("unexpected bus payload variant");
    };
    let input = BabyJubJubAddInput::from(&values);
    assert_eq!((input.p1_addr as u64, input.p2_addr as u64, input.step), (data[A], data[B], step));
    assert_eq!((input.p1, input.p2), (p1, p2));
}
