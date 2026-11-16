use std::panic::{catch_unwind, AssertUnwindSafe};
use zisk_common::{ExtOperationData, OPERATION_BUS_FLOATDOT_DATA_SIZE};
use zisk_common::{OperationBusData, MAX_OPERATION_DATA_SIZE};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::{floatdot_consume_mem_reads, EmulationMode, ZiskOperationType};
use zisk_core::{InstContext, ZiskInstBuilder, ROM_ADDR};

fn replay() -> (ZiskInstBuilder, InstContext) {
    let mut instruction = ZiskInstBuilder::new(ROM_ADDR);
    instruction.op("floatdot").unwrap();
    let mut ctx = InstContext::new();
    ctx.b = 0x3000;
    ctx.step = 77;
    ctx.emulation_mode = EmulationMode::ConsumeMemReads;
    ctx.precompiled.input_data = vec![0x1000, 0x2000, 3, 0, 8, 9 << 32];
    floatdot_consume_mem_reads(&mut ctx);
    (instruction, ctx)
}

fn payload(instruction: &ZiskInstBuilder, ctx: &InstContext) -> Vec<u64> {
    let mut buffer = [0; MAX_OPERATION_DATA_SIZE];
    OperationBusData::write_instruction_payload(&instruction.i, ctx, &mut buffer).to_vec()
}

#[test]
fn dedicated_fp32_payload_preserves_the_complete_fixed_header() {
    let (instruction, ctx) = replay();
    let expected = vec![
        u64::from(ZiskOp::FLOATDOT),
        ZiskOperationType::FloatDot as u64,
        0,
        0x3000,
        77,
        0x1000,
        0x2000,
        3,
        0,
        8,
        9 << 32,
    ];
    assert_eq!(payload(&instruction, &ctx), expected);
    assert_eq!(expected.len(), OPERATION_BUS_FLOATDOT_DATA_SIZE);
    let typed = OperationBusData::from_instruction(&instruction.i, &ctx);
    assert_eq!(OperationBusData::get_op(&typed), ZiskOp::FLOATDOT);
    assert_eq!(OperationBusData::get_op_type(&typed), ZiskOperationType::FloatDot as u64);
    assert_eq!(OperationBusData::get_a(&typed), 0);
    assert_eq!(OperationBusData::get_b(&typed), 0x3000);
    match typed {
        ExtOperationData::OperationFloatDotData(words) => assert_eq!(words.as_slice(), expected),
        _ => panic!("FloatDot must not fall back to a generic operation"),
    }
    assert!(matches!(
        ExtOperationData::<u64>::try_from(expected.as_slice()),
        Ok(ExtOperationData::OperationFloatDotData(_))
    ));
    let ext = [0x3f800000_4b800000, 0x7f800001_cb800000, 0x3f800000_3f800000, 0x7f800001_3f800000];
    assert!(OperationBusData::validate_floatdot_payload(&expected, &ext).is_ok());
    for end in 0..ext.len() {
        assert!(OperationBusData::validate_floatdot_payload(&expected, &ext[..end]).is_err());
    }
    assert!(OperationBusData::validate_floatdot_payload(&expected, &[0; 5]).is_err());
}

#[test]
fn serialized_payload_changes_with_descriptor_step_and_every_fixed_word() {
    let (instruction, ctx) = replay();
    let original = payload(&instruction, &ctx);
    for index in 0..8 {
        let (_, mut ctx) = replay();
        match index {
            0 => ctx.b += 8,
            1 => ctx.step += 1,
            2 => ctx.precompiled.input_data[0] += 8,
            3 => ctx.precompiled.input_data[1] += 8,
            4 => ctx.precompiled.input_data[2] = 2,
            5 => ctx.precompiled.input_data[3] = 1,
            6 => ctx.precompiled.input_data[4] = 9,
            7 => ctx.precompiled.input_data[5] += 1,
            _ => unreachable!(),
        }
        floatdot_consume_mem_reads(&mut ctx);
        assert_ne!(payload(&instruction, &ctx), original, "field {index}");
    }
}

#[test]
fn dedicated_parser_rejects_truncation_reserved_fields_and_wrong_header_width() {
    let (instruction, ctx) = replay();
    let original = payload(&instruction, &ctx);
    for end in 0..original.len() {
        assert!(ExtOperationData::<u64>::try_from(&original[..end]).is_err());
    }
    let mut extra = original.clone();
    extra.push(0);
    assert!(ExtOperationData::<u64>::try_from(extra.as_slice()).is_err());
    for (index, value) in [
        (0, u64::from(ZiskOp::FLOATDOT) + 256),
        (1, 0),
        (2, 1),
        (3, 1 << 32),
        (4, 1 << 36),
        (5, 1),
        (6, u64::MAX),
        (7, 4097),
        (8, 1 << 32),
        (9, 32),
        (10, 1 << 37),
    ] {
        let mut malformed = original.clone();
        malformed[index] = value;
        assert!(ExtOperationData::<u64>::try_from(malformed.as_slice()).is_err(), "field {index}");
    }
    let mut wrong_result = original;
    wrong_result[10] = 0x1f_7fc01234;
    assert!(OperationBusData::validate_floatdot_payload(&wrong_result, &[0; 4]).is_ok());
}

#[test]
fn serializer_rejects_m32_reserved_a_and_inconsistent_replay_state() {
    for case in 0..8 {
        let (mut instruction, mut ctx) = replay();
        match case {
            0 => instruction.i.m32 = true,
            1 => ctx.a = 1 << 32,
            2 => ctx.data_ext_len += 1,
            3 => ctx.c ^= 1,
            4 => ctx.flag = true,
            5 => ctx.precompiled.input_data.push(0),
            6 => instruction.i.op_type = ZiskOperationType::None,
            7 => instruction.i.op = ZiskOp::ADD,
            _ => unreachable!(),
        }
        assert!(catch_unwind(AssertUnwindSafe(|| payload(&instruction, &ctx))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| OperationBusData::from_instruction(
            &instruction.i,
            &ctx
        )))
        .is_err());
    }
}

#[test]
fn generic_value_serializer_cannot_emit_an_incomplete_fp32_call() {
    for (op, op_type) in [
        (ZiskOp::FLOATDOT, ZiskOperationType::FloatDot as u64),
        (ZiskOp::FLOATDOT, ZiskOperationType::None as u64),
        (ZiskOp::ADD, ZiskOperationType::FloatDot as u64),
    ] {
        let mut pending = std::collections::VecDeque::new();
        assert!(catch_unwind(AssertUnwindSafe(|| {
            OperationBusData::from_values(op, op_type, 0, 0x3000, &mut pending);
        }))
        .is_err());
        assert!(pending.is_empty());
    }
}

#[test]
fn incomplete_fp32_payload_cannot_fall_back_to_a_generic_operation() {
    let mut builder = ZiskInstBuilder::new(ROM_ADDR);
    builder.op("floatdot").unwrap();
    let ctx = InstContext::new();
    assert!(catch_unwind(AssertUnwindSafe(|| OperationBusData::from_instruction(
        &builder.i, &ctx
    )))
    .is_err());
    let mut buffer = [0; MAX_OPERATION_DATA_SIZE];
    assert!(catch_unwind(AssertUnwindSafe(|| {
        OperationBusData::write_instruction_payload(&builder.i, &ctx, &mut buffer);
    }))
    .is_err());
}
