//! Layout test for the mem inputs of `blake2b`, called with two direct operands (the state, which
//! receives the result, at `a` and the input block at `b`) plus the round index as the static
//! argument of the instruction, carried as the first word of the payload. Nothing is read from
//! memory for the index or the addresses.

use proofman_fields::Goldilocks;
use zisk_common::{ExtOperationData, A, B, OPERATION_BUS_BLAKE2B_DATA_SIZE, STEP};
use zisk_core::blake2br;
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::ZiskOperationType;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use crate::{Blake2bInput, Blake2bSM};

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
fn blake2b_mem_inputs_take_the_operands_from_a_and_b_and_the_index_from_the_payload() {
    let (a, b, step, index) = (0xA001_0000u64, 0xA002_0080u64, 77u64, 7u64);
    let state: [u64; 16] = std::array::from_fn(|i| 1 + i as u64);
    let input: [u64; 16] = std::array::from_fn(|i| 101 + i as u64);

    // Bus payload: [op, op_type, a, b, step, index, state[16], input[16]]
    let mut data =
        vec![ZiskOp::BLAKE2B as u64, ZiskOperationType::Blake2b as u64, a, b, step, index];
    data.extend_from_slice(&state);
    data.extend_from_slice(&input);
    assert_eq!(data.len(), OPERATION_BUS_BLAKE2B_DATA_SIZE);

    let mut new_state = state;
    blake2br(index, &mut new_state, &input);

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
    <Blake2bSM<Goldilocks> as PrecompileMemInputs>::generate(
        data[B] as u32,
        data[STEP],
        &data,
        false,
        &mut recorder,
    );
    assert_eq!(recorder.rows, expected.rows);

    // The witness input decodes the addresses from a and b, and the index from the payload.
    let ext: ExtOperationData<u64> = data.as_slice().try_into().unwrap();
    let ExtOperationData::OperationBlake2bData(values) = ext else {
        panic!("unexpected bus payload variant");
    };
    let decoded = Blake2bInput::from(&values);
    assert_eq!((decoded.state_addr as u64, decoded.input_addr as u64), (data[A], data[B]));
    assert_eq!((decoded.index, decoded.step_main), (index, step));
    assert_eq!((decoded.state, decoded.input), (state, input));
}
