//! Replay transport regression; recording payloads is not a cryptographic bus proof.
use std::path::Path;
use zisk_common::{
    BusId, DataBusTrait, OperationBusData, A, B, OP, OPERATION_BUS_FLOATDOT_DATA_SIZE,
    OPERATION_BUS_ID, OP_TYPE, STEP,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType, ZiskRom};
use ziskemu::{Emu, EmuOptions, ZiskEmulator};

const CHUNK_SIZE: u64 = 256;

#[derive(Debug, PartialEq, Eq)]
struct Call {
    header: [u64; OPERATION_BUS_FLOATDOT_DATA_SIZE],
    words: Vec<u64>,
}

#[derive(Default)]
struct Recorder {
    calls: Vec<Call>,
    chunk_start: u64,
    chunk_end: u64,
}

impl DataBusTrait<u64, ()> for Recorder {
    fn write_to_bus(&mut self, bus_id: BusId, data: &[u64], data_ext: &[u64]) -> bool {
        if bus_id == OPERATION_BUS_ID && data.get(OP) == Some(&u64::from(ZiskOp::FLOATDOT)) {
            OperationBusData::validate_floatdot_payload(data, data_ext).unwrap();
            assert_eq!(data[OP_TYPE], ZiskOperationType::FloatDot as u64);
            assert_eq!(data[A], 0);
            assert!((self.chunk_start..self.chunk_end).contains(&data[STEP]));
            self.calls.push(Call { header: data.try_into().unwrap(), words: data_ext.to_vec() });
        }
        true
    }

    fn on_close(&mut self) {}

    fn into_devices(self, _: bool) -> Vec<(usize, ())> {
        Vec::new()
    }
}

// Capture addresses, steps and packed words from ordinary memory execution independently
// of minimal-trace generation and all three replay implementations.
fn memory_calls(rom: &ZiskRom, options: &EmuOptions) -> Vec<Call> {
    let mut emu = Emu::new(rom);
    emu.ctx = emu.create_emu_context(Vec::new(), options);
    let mut calls = Vec::new();
    while !emu.terminated() {
        assert!(emu.number_of_steps() < options.max_steps);
        let instruction = rom.get_instruction(emu.ctx.inst_ctx.pc);
        let fp32 = instruction.op == ZiskOp::FLOATDOT;
        let step = emu.number_of_steps();
        emu.step_fast();
        if fp32 {
            let ctx = &emu.ctx.inst_ctx;
            assert_eq!(ctx.a, 0);
            assert!(!ctx.flag);
            let descriptor: [u64; 5] =
                std::array::from_fn(|i| ctx.mem.read(ctx.b + u64::try_from(i).unwrap() * 8, 8));
            let mut header = [0; OPERATION_BUS_FLOATDOT_DATA_SIZE];
            header[..5].copy_from_slice(&[
                u64::from(ZiskOp::FLOATDOT),
                ZiskOperationType::FloatDot as u64,
                ctx.a,
                ctx.b,
                step,
            ]);
            header[5..10].copy_from_slice(&descriptor);
            header[10] = ctx.c;
            assert!(descriptor[2] <= 4096);
            let mut words = Vec::new();
            for &word in &descriptor[..2] {
                if descriptor[4] >> 8 == 0 {
                    for i in 0..descriptor[2].div_ceil(2) {
                        words.push(ctx.mem.read(word + 8 * i, 8));
                    }
                } else {
                    // 16-bit lanes: the enclosing word of each strided element.
                    let (base, stride) = (word & 0xffff_ffff, word >> 32);
                    for i in 0..descriptor[2] {
                        words.push(ctx.mem.read((base + 2 * stride * i) & !7, 8));
                    }
                }
            }
            OperationBusData::validate_floatdot_payload(&header, &words).unwrap();
            calls.push(Call { header, words });
        }
    }
    assert!(!emu.ctx.inst_ctx.error);
    calls
}

#[test]
#[ignore = "requires rebuilt target/fp32/artifacts/precompile.elf"]
fn actual_fp32_guest_replays_all_public_paths_across_chunks() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/fp32/artifacts/precompile.elf");
    let elf = std::fs::read(path)
        .expect("build the guest ELFs first: node precompiles/floatdot/tests/run_guests.cjs");
    let rom = zisk_riscv::elf2rom(&elf).expect("transpile FP32 guest");
    let options =
        EmuOptions { chunk_size: Some(CHUNK_SIZE), max_steps: 1_000_000, ..Default::default() };
    let expected = memory_calls(&rom, &options);
    assert_eq!(expected.len(), 5);
    assert_eq!(expected.iter().map(|call| call.header[7]).collect::<Vec<_>>(), [3, 2, 3, 4096, 0]);
    assert_eq!(
        expected.iter().map(|call| call.header[10]).collect::<Vec<_>>(),
        [1 << 32, (3 << 32) | 1, 0xbec00000, 0x45800000, (8 << 32) | 0xff800001,]
    );
    assert!(expected.windows(2).all(|pair| pair[0].header[STEP] < pair[1].header[STEP]));
    assert_ne!(expected[0].header[STEP] / CHUNK_SIZE, expected[4].header[STEP] / CHUNK_SIZE);
    assert!(expected.iter().all(|call| call.header[B] != 0));
    assert_eq!(
        expected[0].words,
        [0x3f800000_4b800000, 0x7f800001_cb800000, 0x3f800000_3f800000, 0x7f800001_3f800000,]
    );
    assert!(expected[3].words.iter().all(|&word| word == 0x3f800000_3f800000));
    assert_eq!(&expected[4].header[8..10], &[0xff800001, 8]);
    for (call, words) in expected.iter().zip([4, 2, 4, 4096, 0]) {
        assert_eq!(call.words.len(), words);
    }

    let large_chunk = usize::try_from(expected[3].header[STEP] / CHUNK_SIZE).unwrap();
    replay_matches(&rom, &options, &expected, large_chunk, 6 + 4096);
}

/// Every public replay path must reproduce the independently captured calls across chunks.
fn replay_matches(
    rom: &ZiskRom,
    options: &EmuOptions,
    expected: &[Call],
    large: usize,
    reads: usize,
) {
    let traces = ZiskEmulator::compute_minimal_traces(rom, &[], options, 2)
        .expect("FP32 minimal-trace generation must succeed once the proof-mode gate is opened");
    assert!(traces.len() > 2);
    let mut next_step = 0;
    for chunk in &traces {
        assert_eq!(chunk.start_state.step, next_step);
        assert!((1..=CHUNK_SIZE).contains(&chunk.steps));
        next_step += chunk.steps;
    }
    assert!(traces.last().unwrap().end);
    assert!(traces[large].mem_reads.len() >= reads);

    for path in ["with memory ops", "without memory ops", "collector replay"] {
        let mut bus = Recorder::default();
        for chunk in &traces {
            bus.chunk_start = chunk.start_state.step;
            bus.chunk_end = bus.chunk_start + chunk.steps;
            let mut emu = Emu::new_with_memory_data(rom, path == "with memory ops");
            match path {
                "with memory ops" => emu.process_emu_trace(chunk, &mut bus, true),
                "without memory ops" => emu.process_emu_trace(chunk, &mut bus, false),
                _ => emu.process_emu_traces(chunk, &mut bus),
            }
            assert_eq!(emu.number_of_steps(), bus.chunk_end, "{path}");
            assert_eq!(emu.ctx.inst_ctx.c, chunk.last_c, "{path}");
            assert_eq!(emu.terminated(), chunk.end, "{path}");
        }
        assert_eq!(
            bus.calls, expected,
            "{path} must preserve every descriptor, step and operand word"
        );
    }
}

#[test]
#[ignore = "requires rebuilt target/fp32/artifacts/floatdot-precompile.elf"]
fn actual_mixed_input_guest_replays_all_public_paths_across_chunks() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/fp32/artifacts/floatdot-precompile.elf");
    let elf = std::fs::read(path)
        .expect("build the guest ELFs first: node precompiles/floatdot/tests/run_guests.cjs");
    let rom = zisk_riscv::elf2rom(&elf).expect("transpile mixed guest");
    let options =
        EmuOptions { chunk_size: Some(CHUNK_SIZE), max_steps: 1_000_000, ..Default::default() };
    let expected = memory_calls(&rom, &options);
    let field = |i: usize| expected.iter().map(|call| call.header[i]).collect::<Vec<_>>();
    assert_eq!(field(7), [128, 33, 7, 64, 4096, 3, 0, 3]);
    assert_eq!(field(9).iter().map(|f| f >> 8).collect::<Vec<_>>(), [1, 1, 2, 2, 1, 1, 1, 0]);
    // Strides: Q·K (1,1), P·V (1,1024), FP16 (3,0 broadcast), legacy FP32 none.
    let strides = |i: usize| field(i).iter().map(|w| w >> 32).collect::<Vec<_>>();
    assert_eq!(strides(5), [1, 1, 3, 1, 1, 1, 1, 0]);
    assert_eq!(strides(6), [1, 1024, 0, 1, 1, 1, 1, 0]);
    for (call, words) in expected.iter().zip([256, 66, 14, 128, 8192, 6, 0, 4]) {
        assert_eq!(call.words.len(), words);
    }
    // V element t is at byte 2048t + 10 of an aligned buffer: lane 1; other lanes are ignored.
    assert!(expected[1].words[33..].iter().all(|w| w & !(0xffff << 16) == 0x7fa1_7fa1_0000_7fa1));
    // The tail call continues from the maximum-length call's bits and flags.
    assert_eq!(expected[5].header[8], expected[4].header[10] & 0xffff_ffff);
    assert_eq!(expected[5].header[9] & 31, expected[4].header[10] >> 32);
    assert!(expected.windows(2).all(|pair| pair[0].header[STEP] < pair[1].header[STEP]));
    let large = usize::try_from(expected[4].header[STEP] / CHUNK_SIZE).unwrap();
    replay_matches(&rom, &options, &expected, large, 6 + 8192);
}
