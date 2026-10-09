//! Layout test for the mem inputs of `sha256f`, called with two direct operands: the state (which
//! receives the result) lives at `a`, the input block at `b`, and there is no parameter struct.

use proofman_fields::Goldilocks;
use zisk_common::{ExtOperationData, A, B, OPERATION_BUS_SHA256F_DATA_SIZE, STEP};
use zisk_core::sha256f;
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::ZiskOperationType;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use crate::{Sha256fInput, Sha256fSM};

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
fn sha256f_mem_inputs_take_the_operands_from_a_and_b() {
    let (a, b, step) = (0xA001_0000u64, 0xA002_0040u64, 77u64);
    let state: [u64; 4] = [1, 2, 3, 4];
    let input: [u64; 8] = [11, 12, 13, 14, 15, 16, 17, 18];

    // Bus payload: [op, op_type, a, b, step, state[4], input[8]]
    let mut data = vec![ZiskOp::SHA256 as u64, ZiskOperationType::Sha256 as u64, a, b, step];
    data.extend_from_slice(&state);
    data.extend_from_slice(&input);
    assert_eq!(data.len(), OPERATION_BUS_SHA256F_DATA_SIZE);

    let mut new_state = state;
    sha256f(&mut new_state, &input);

    // Reads of the state at a and of the input at b, then the new state written back at a.
    let mut expected = Recorder::default();
    for (i, value) in state.iter().enumerate() {
        MemBusHelpers::mem_aligned_read(a as u32 + 8 * i as u32, step, *value, &mut expected);
    }
    for (i, value) in input.iter().enumerate() {
        MemBusHelpers::mem_aligned_read(b as u32 + 8 * i as u32, step, *value, &mut expected);
    }
    for (i, value) in new_state.iter().enumerate() {
        MemBusHelpers::mem_aligned_write(a as u32 + 8 * i as u32, step, *value, &mut expected);
    }

    let mut recorder = Recorder::default();
    <Sha256fSM<Goldilocks> as PrecompileMemInputs>::generate(
        data[B] as u32,
        data[STEP],
        &data,
        false,
        &mut recorder,
    );
    assert_eq!(recorder.rows, expected.rows);

    // The witness input decodes the addresses from a and b too.
    let ext: ExtOperationData<u64> = data.as_slice().try_into().unwrap();
    let ExtOperationData::OperationSha256Data(values) = ext else {
        panic!("unexpected bus payload variant");
    };
    let decoded = Sha256fInput::from(&values);
    assert_eq!((decoded.state_addr as u64, decoded.input_addr as u64), (data[A], data[B]));
    assert_eq!((decoded.state, decoded.input, decoded.step_main), (state, input, step));
}
