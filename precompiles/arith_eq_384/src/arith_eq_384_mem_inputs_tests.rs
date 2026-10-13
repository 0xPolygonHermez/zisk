//! Layout tests for the mem inputs of the ops called with two direct operands: the first operand
//! (which receives the result) lives at `a`, the second one at `b`, and there is no parameter
//! struct, so no indirection is read.

use proofman_fields::Goldilocks;
use zisk_common::{
    ExtOperationData, A, B, OPERATION_BUS_BLS12_381_COMPLEX_SUB_DATA_SIZE,
    OPERATION_BUS_BLS12_381_CURVE_ADD_DATA_SIZE, STEP,
};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::ZiskOperationType;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use crate::{
    executors, ArithEq384SM, Bls12_381ComplexSubInput, Bls12_381CurveAddInput,
    ARITH_EQ_384_U64S_DOUBLE,
};

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

/// Bus payload of a direct a/b op: `[op, op_type, a, b, step, p1..., p2...]`.
fn direct_ab_data(op: u8, a: u64, b: u64, step: u64, p1: &[u64], p2: &[u64]) -> Vec<u64> {
    let mut data = vec![op as u64, ZiskOperationType::ArithEq384 as u64, a, b, step];
    data.extend_from_slice(p1);
    data.extend_from_slice(p2);
    data
}

/// Reads of `p1` at `a`, reads of `p2` at `b`, and the result written back at `a`.
fn expected_rows(a: u64, b: u64, step: u64, p1: &[u64], p2: &[u64], p3: &[u64]) -> Vec<[u64; 7]> {
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
    expected.rows
}

fn point(first: u64) -> [u64; ARITH_EQ_384_U64S_DOUBLE] {
    std::array::from_fn(|i| first + i as u64)
}

#[test]
fn bls12_381_curve_add_mem_inputs_take_the_operands_from_a_and_b() {
    let (a, b, step) = (0xA001_0000u64, 0xA002_0060u64, 77u64);
    let p1 = point(1);
    let p2 = point(101);
    let data = direct_ab_data(ZiskOp::BLS12_381_CURVE_ADD, a, b, step, &p1, &p2);
    assert_eq!(data.len(), OPERATION_BUS_BLS12_381_CURVE_ADD_DATA_SIZE);

    let mut p3 = [0u64; ARITH_EQ_384_U64S_DOUBLE];
    executors::Bls12_381Curve::calculate_add(&p1, &p2, &mut p3);

    let mut recorder = Recorder::default();
    <ArithEq384SM<Goldilocks> as PrecompileMemInputs>::generate(
        data[B] as u32,
        data[STEP],
        &data,
        false,
        &mut recorder,
    );
    assert_eq!(recorder.rows, expected_rows(a, b, step, &p1, &p2, &p3));

    // The witness input decodes the addresses from a and b too.
    let ext: ExtOperationData<u64> = data.as_slice().try_into().unwrap();
    let ExtOperationData::OperationBls12_381CurveAddData(values) = ext else {
        panic!("unexpected bus payload variant");
    };
    let input = Bls12_381CurveAddInput::from(&values);
    assert_eq!((input.p1_addr as u64, input.p2_addr as u64, input.step), (data[A], data[B], step));
    assert_eq!((input.p1, input.p2), (p1, p2));
}

#[test]
fn bls12_381_complex_sub_mem_inputs_take_the_operands_from_a_and_b() {
    let (a, b, step) = (0xA100_00C0u64, 0xA100_0000u64, 5u64);
    let f1 = point(201);
    let f2 = point(301);
    let data = direct_ab_data(ZiskOp::BLS12_381_COMPLEX_SUB, a, b, step, &f1, &f2);
    assert_eq!(data.len(), OPERATION_BUS_BLS12_381_COMPLEX_SUB_DATA_SIZE);

    let mut f3 = [0u64; ARITH_EQ_384_U64S_DOUBLE];
    executors::Bls12_381Complex::calculate_sub(&f1, &f2, &mut f3);

    let mut recorder = Recorder::default();
    <ArithEq384SM<Goldilocks> as PrecompileMemInputs>::generate(
        data[B] as u32,
        data[STEP],
        &data,
        false,
        &mut recorder,
    );
    assert_eq!(recorder.rows, expected_rows(a, b, step, &f1, &f2, &f3));

    let ext: ExtOperationData<u64> = data.as_slice().try_into().unwrap();
    let ExtOperationData::OperationBls12_381ComplexSubData(values) = ext else {
        panic!("unexpected bus payload variant");
    };
    let input = Bls12_381ComplexSubInput::from(&values);
    assert_eq!((input.f1_addr as u64, input.f2_addr as u64), (a, b));
    assert_eq!((input.f1, input.f2), (f1, f2));
}
