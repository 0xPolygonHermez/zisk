//! Layout tests for the mem inputs of the ops called with two direct operands: the first operand
//! (which receives the result) lives at `a`, the second one at `b`, and there is no parameter
//! struct, so no indirection is read.
//! Declared from `lib.rs` via `#[cfg(test)] #[path = "tests/arith_eq_mem_inputs_tests.rs"]`.

use proofman_fields::Goldilocks;
use zisk_common::{
    ExtOperationData, A, B, OPERATION_BUS_BN254_COMPLEX_MUL_DATA_SIZE,
    OPERATION_BUS_SECP256K1_ADD_DATA_SIZE, STEP,
};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::ZiskOperationType;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use crate::{executors, ArithEqSM, Bn254ComplexMulInput, Secp256k1AddInput};

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
    let mut data = vec![op as u64, ZiskOperationType::ArithEq as u64, a, b, step];
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

#[test]
fn secp256k1_add_mem_inputs_take_the_operands_from_a_and_b() {
    let (a, b, step) = (0xA001_0000u64, 0xA002_0040u64, 77u64);
    let p1: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let p2: [u64; 8] = [11, 12, 13, 14, 15, 16, 17, 18];
    let data = direct_ab_data(ZiskOp::SECP256K1_ADD, a, b, step, &p1, &p2);
    assert_eq!(data.len(), OPERATION_BUS_SECP256K1_ADD_DATA_SIZE);

    let mut p3 = [0u64; 8];
    executors::Secp256k1::calculate_add(&p1, &p2, &mut p3);

    let mut recorder = Recorder::default();
    <ArithEqSM<Goldilocks> as PrecompileMemInputs>::generate(
        data[B] as u32,
        data[STEP],
        &data,
        false,
        &mut recorder,
    );
    assert_eq!(recorder.rows, expected_rows(a, b, step, &p1, &p2, &p3));

    // The witness input decodes the addresses from a and b too.
    let ext: ExtOperationData<u64> = data.as_slice().try_into().unwrap();
    let ExtOperationData::OperationSecp256k1AddData(values) = ext else {
        panic!("unexpected bus payload variant");
    };
    let input = Secp256k1AddInput::from(&values);
    assert_eq!((input.p1_addr as u64, input.p2_addr as u64, input.step), (data[A], data[B], step));
    assert_eq!((input.p1, input.p2), (p1, p2));
}

#[test]
fn bn254_complex_mul_mem_inputs_take_the_operands_from_a_and_b() {
    let (a, b, step) = (0xA100_0080u64, 0xA100_0000u64, 5u64);
    let f1: [u64; 8] = [21, 22, 23, 24, 25, 26, 27, 28];
    let f2: [u64; 8] = [31, 32, 33, 34, 35, 36, 37, 38];
    let data = direct_ab_data(ZiskOp::BN254_COMPLEX_MUL, a, b, step, &f1, &f2);
    assert_eq!(data.len(), OPERATION_BUS_BN254_COMPLEX_MUL_DATA_SIZE);

    let mut f3 = [0u64; 8];
    executors::Bn254Complex::calculate_mul(&f1, &f2, &mut f3);

    let mut recorder = Recorder::default();
    <ArithEqSM<Goldilocks> as PrecompileMemInputs>::generate(
        data[B] as u32,
        data[STEP],
        &data,
        false,
        &mut recorder,
    );
    assert_eq!(recorder.rows, expected_rows(a, b, step, &f1, &f2, &f3));

    let ext: ExtOperationData<u64> = data.as_slice().try_into().unwrap();
    let ExtOperationData::OperationBn254ComplexMulData(values) = ext else {
        panic!("unexpected bus payload variant");
    };
    let input = Bn254ComplexMulInput::from(&values);
    assert_eq!((input.f1_addr as u64, input.f2_addr as u64), (a, b));
    assert_eq!((input.f1, input.f2), (f1, f2));
}
