use std::panic::{catch_unwind, AssertUnwindSafe};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::{EmulationMode, InstContext};

const BASE: u64 = 0xa1000000;
fn fixture(words: [u64; 5]) -> InstContext {
    let mut ctx = InstContext::new();
    ctx.mem.add_write_section(BASE, 0x10000);
    ctx.b = BASE;
    for (i, w) in words.into_iter().enumerate() {
        ctx.mem.write(BASE + i as u64 * 8, w, 8);
    }
    ctx
}
fn run(ctx: &mut InstContext) {
    ZiskOp::try_from_name("floatdot").unwrap().call(ctx);
}

#[test]
fn fp32_dot_memory_order_odd_tail_alias_and_chunking() {
    let mut ctx = fixture([BASE + 0x100, BASE + 0x200, 3, 0, 8]);
    ctx.mem.write(BASE + 0x100, 0x3f800000_4b800000, 8);
    ctx.mem.write(BASE + 0x108, 0x7f800001_cb800000, 8); // ignored odd tail must not set NV
    ctx.mem.write(BASE + 0x200, 0x3f800000_3f800000, 8);
    ctx.mem.write(BASE + 0x208, 0x7f800001_3f800000, 8);
    run(&mut ctx);
    assert_eq!((ctx.c, ctx.flag), (9u64 << 32, false));
    ctx.mem.write(BASE + 16, 2, 8);
    run(&mut ctx);
    let carry = ctx.c;
    ctx.mem.write(BASE, BASE + 0x108, 8);
    ctx.mem.write(BASE + 8, BASE + 0x208, 8);
    ctx.mem.write(BASE + 16, 1, 8);
    ctx.mem.write(BASE + 24, carry as u32 as u64, 8);
    ctx.mem.write(BASE + 32, carry >> 32, 8);
    run(&mut ctx);
    assert_eq!(ctx.c, 9u64 << 32);
    // Repeated reads from the same memory are valid.
    let mut alias = fixture([BASE + 0x100, BASE + 0x100, 2, 0, 0]);
    alias.mem.write(BASE + 0x100, 0xc0000000_3f800000, 8);
    run(&mut alias);
    assert_eq!(alias.c, 0x40a00000);
}

#[test]
fn fp32_dot_empty_preserves_bits_without_vector_reads() {
    for flags in 0..32 {
        let mut ctx = fixture([0, 0, 0, 0xff800001, flags]);
        run(&mut ctx);
        assert_eq!(ctx.c, (flags << 32) | 0xff800001);
    }
}

#[test]
fn fp32_dot_rejects_malformed_memory_contracts() {
    ZiskOp::try_from_name("floatdot").unwrap();
    for words in [
        [BASE + 4, BASE, 1, 0, 0],
        [BASE, BASE + 4, 1, 0, 0],
        [1u64 << 32, BASE, 0, 0, 0],
        [BASE, 1u64 << 32, 0, 0, 0],
        [0xfffffff8, BASE, 3, 0, 0],
        [BASE, 0xfffffff8, 3, 0, 0],
        [BASE, BASE, 4097, 0, 0],
        [BASE, BASE, u64::MAX, 0, 0],
        [BASE, BASE, 0, 1u64 << 32, 0],
        [BASE, BASE, 0, 0, 32],
        [0, 0, 1, 0, 0],
    ] {
        let mut ctx = fixture(words);
        assert!(catch_unwind(AssertUnwindSafe(|| run(&mut ctx))).is_err(), "{words:?}");
    }
    for addr in [BASE + 4, 0xfffffff8, u64::MAX] {
        let mut ctx = fixture([0; 5]);
        ctx.b = addr;
        assert!(catch_unwind(AssertUnwindSafe(|| run(&mut ctx))).is_err());
    }
    let mut ctx = fixture([0; 5]);
    ctx.a = 1;
    assert!(catch_unwind(AssertUnwindSafe(|| run(&mut ctx))).is_err());
}

#[test]
fn fp32_public_replay_preserves_empty_initial_bits() {
    let mut ctx = fixture([0, 0, 0, 0xff800001, 31]);
    ctx.emulation_mode = EmulationMode::GenerateMemReads;
    run(&mut ctx);
    assert_eq!(ctx.precompiled.input_data, [0, 0, 0, 0xff800001, 31, 0x1f_ff800001]);
    ctx.emulation_mode = EmulationMode::ConsumeMemReads;
    ctx.c = 0;
    run(&mut ctx);
    assert_eq!((ctx.c, ctx.flag, ctx.data_ext_len), (0x1f_ff800001, false, 0));
}

#[test]
fn fp32_dot_all_assembly_modes_fail_closed() {
    use zisk_core::{
        AsmGenerationMethod, ZiskInstBuilder, ZiskRom, ZiskRom2Asm, ROM_ADDR, ROM_ENTRY,
    };
    let mut instruction = ZiskInstBuilder::new(ROM_ADDR);
    instruction.src_a("imm", 0, false);
    instruction.src_b("imm", BASE, false);
    instruction.op("floatdot").unwrap();
    instruction.j(0, 4);
    let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };
    instruction.build(&mut rom);
    rom.optimize_instruction_lookup().unwrap();
    for mode in [
        AsmGenerationMethod::AsmFast,
        AsmGenerationMethod::AsmMinimalTraces,
        AsmGenerationMethod::AsmRomHistogram,
        AsmGenerationMethod::AsmMemOp,
        AsmGenerationMethod::AsmMemOpLight,
    ] {
        let failure = catch_unwind(AssertUnwindSafe(|| {
            ZiskRom2Asm::save_to_asm(&rom, &mut String::new(), mode, false, false, false)
        }))
        .unwrap_err();
        let message = failure
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| failure.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(message.contains("floatdot Assembly support is not implemented"), "{message}");
    }
}

#[test]
fn fp32_dot_maximum_length_and_descriptor_overlap() {
    let mut ctx = fixture([BASE + 0x100, BASE + 0x100, 4096, 0, 0]);
    for i in 0..2048 {
        ctx.mem.write(BASE + 0x100 + 8 * i, 0x3f800000_3f800000, 8);
    }
    run(&mut ctx);
    assert_eq!(ctx.c, 0x45800000);
    let mut ctx = fixture([BASE, BASE, 2, 0, 0]);
    run(&mut ctx);
    assert_eq!(ctx.c, 0x02800000);
}

#[test]
fn fp32_does_not_reinterpret_known_integer_dot_opcode() {
    assert_eq!(ZiskOp::try_from_code(0xc2).unwrap().name(), "dot_i8");
    assert_ne!(ZiskOp::FloatDot.code(), 0xc2);
}

#[test]
fn appended_fp32_operation_is_not_omitted_by_replay_classification() {
    use zisk_core::ZiskOperationType::*;
    for op in [Arith, Binary, Sha256, Dma, FloatDot] {
        assert!(op.is_proof_operation());
    }
    for op in [None, Internal, FcallParam, Fcall, FcallGet, Profile, DotI8] {
        assert!(!op.is_proof_operation());
    }
}

#[test]
fn fp32_transport_uses_six_fixed_words_before_extended_vectors() {
    use zisk_core::{
        floatdot_consume_mem_reads, floatdot_generate_mem_reads, ZiskInstBuilder, ROM_ADDR,
    };
    let mut ctx = fixture([BASE + 0x100, BASE + 0x200, 3, 0, 8]);
    ctx.mem.write(BASE + 0x100, 0x3f800000_4b800000, 8);
    ctx.mem.write(BASE + 0x108, 0x7f800001_cb800000, 8);
    ctx.mem.write(BASE + 0x200, 0x3f800000_3f800000, 8);
    ctx.mem.write(BASE + 0x208, 0x7f800001_3f800000, 8);
    ctx.step = 77;
    ctx.emulation_mode = EmulationMode::GenerateMemReads;
    ctx.precompiled.input_data = vec![u64::MAX; 40];
    ctx.precompiled.output_data = vec![1];
    floatdot_generate_mem_reads(&mut ctx);
    let payload = vec![
        BASE + 0x100,
        BASE + 0x200,
        3,
        0,
        8,
        9 << 32,
        0x3f800000_4b800000,
        0x7f800001_cb800000,
        0x3f800000_3f800000,
        0x7f800001_3f800000,
    ];
    assert_eq!(ctx.precompiled.input_data, payload);
    assert_eq!(ctx.precompiled.step, 77);
    assert!(ctx.precompiled.output_data.is_empty());
    assert_eq!((ctx.c, ctx.flag), (9 << 32, false));
    let mut instruction = ZiskInstBuilder::new(ROM_ADDR);
    instruction.op("floatdot").unwrap();
    assert_eq!(instruction.i.input_size, 48);
    assert_eq!(instruction.i.min_trace_words(false), 6);
    let mut consume = InstContext::new();
    consume.b = BASE;
    consume.step = 77;
    consume.emulation_mode = EmulationMode::ConsumeMemReads;
    consume.precompiled.input_data = payload[..6].to_vec();
    floatdot_consume_mem_reads(&mut consume);
    assert_eq!(consume.data_ext_len, 4);
    assert_eq!((consume.c, consume.flag), (9 << 32, false));
    assert_eq!(
        usize::try_from(instruction.i.min_trace_words(false)).unwrap() + consume.data_ext_len,
        payload.len()
    );
    assert_eq!(consume.precompiled.input_data, payload[..6]);
}

#[test]
fn fp32_transport_zero_and_max_lengths_have_exact_counts() {
    use zisk_core::{
        floatdot_consume_mem_reads, floatdot_generate_mem_reads, validate_floatdot_replay_header,
    };
    for n in [0, 1, 2, 4095, 4096] {
        let mut ctx = fixture([BASE + 0x100, BASE + 0x100, n, 0xff800001, 31]);
        ctx.emulation_mode = EmulationMode::GenerateMemReads;
        floatdot_generate_mem_reads(&mut ctx);
        let expected = 2 * usize::try_from(n).unwrap().div_ceil(2);
        assert_eq!(ctx.precompiled.input_data.len(), 6 + expected);
        let header =
            validate_floatdot_replay_header(0, BASE, 0, &ctx.precompiled.input_data[..6]).unwrap();
        assert_eq!(header.memory_read_count(), 5 + expected);
        ctx.precompiled.input_data.truncate(6);
        ctx.emulation_mode = EmulationMode::ConsumeMemReads;
        ctx.data_ext_len = usize::MAX;
        floatdot_consume_mem_reads(&mut ctx);
        assert_eq!(ctx.data_ext_len, expected);
        if n == 0 {
            assert_eq!(ctx.c, 0x1f_ff800001);
        }
    }
}

#[test]
fn fp32_consume_rejects_malformed_headers_before_using_claimed_result() {
    use zisk_core::floatdot_consume_mem_reads;
    let valid = [BASE + 0x100, BASE + 0x200, 3, 0, 8, 9 << 32];
    for (index, value) in
        [(0, BASE + 4), (1, 1 << 32), (2, u64::MAX), (3, 1 << 32), (4, 32), (5, 1 << 37)]
    {
        let mut ctx = InstContext::new();
        ctx.b = BASE;
        ctx.c = 123;
        ctx.emulation_mode = EmulationMode::ConsumeMemReads;
        ctx.precompiled.input_data = valid.to_vec();
        ctx.precompiled.input_data[index] = value;
        assert!(catch_unwind(AssertUnwindSafe(|| floatdot_consume_mem_reads(&mut ctx))).is_err());
        assert_eq!(ctx.c, 123);
    }
    for len in [0, 1, 2, 3, 4, 5, 7, 10] {
        let mut ctx = InstContext::new();
        ctx.b = BASE;
        ctx.emulation_mode = EmulationMode::ConsumeMemReads;
        ctx.precompiled.input_data = valid.to_vec();
        ctx.precompiled.input_data.resize(len, 0);
        assert!(catch_unwind(AssertUnwindSafe(|| floatdot_consume_mem_reads(&mut ctx))).is_err());
    }
    for (a, b, step) in [(1, BASE, 0), (0, BASE + 4, 0), (0, 1 << 32, 0), (0, BASE, 1 << 36)] {
        let mut ctx = InstContext::new();
        ctx.a = a;
        ctx.b = b;
        ctx.step = step;
        ctx.emulation_mode = EmulationMode::ConsumeMemReads;
        ctx.precompiled.input_data = valid.to_vec();
        assert!(catch_unwind(AssertUnwindSafe(|| floatdot_consume_mem_reads(&mut ctx))).is_err());
    }
}

#[test]
fn fp32_generate_reads_only_five_descriptor_words_for_an_empty_call() {
    use zisk_core::floatdot_generate_mem_reads;
    let mut ctx = InstContext::new();
    ctx.b = BASE;
    ctx.emulation_mode = EmulationMode::GenerateMemReads;
    let descriptor = [0xfffffff8u64, 0, 0, 0xff800001, 31];
    ctx.mem.add_write_section(BASE, 40);
    for (i, word) in descriptor.into_iter().enumerate() {
        ctx.mem.write(BASE + u64::try_from(i).unwrap() * 8, word, 8);
    }
    floatdot_generate_mem_reads(&mut ctx);
    assert_eq!(ctx.precompiled.input_data, [0xfffffff8, 0, 0, 0xff800001, 31, 0x1f_ff800001]);
    assert_eq!(ctx.c, 0x1f_ff800001);
}

/// Write 16-bit lanes at `base + 2*stride*i`, leaving other lanes as a NaN pattern.
fn put16(ctx: &mut InstContext, base: u64, stride: u64, values: &[u16]) {
    for (i, &h) in values.iter().enumerate() {
        let address = base + 2 * stride * i as u64;
        let word = address & !7;
        let shift = 16 * ((address >> 1) & 3);
        let old = ctx.mem.read(word, 8);
        ctx.mem.write(word, (old & !(0xffff << shift)) | (u64::from(h) << shift), 8);
    }
}

#[test]
fn bf16_and_fp16_read_strided_lanes_and_replay_one_word_per_element() {
    use zisk_core::{floatdot_consume_mem_reads, floatdot_generate_mem_reads};
    let (x, y) = (BASE + 0x102, BASE + 0x1006);
    let mut ctx = fixture([x | 1 << 32, y | 1024 << 32, 3, 0, 1 << 8]);
    for word in (0x100..0x110).step_by(8).chain((0x1000..0x2010).step_by(8)) {
        ctx.mem.write(BASE + word, 0x7fa1_7fa1_7fa1_7fa1, 8);
    }
    put16(&mut ctx, x, 1, &[0x3f80, 0x4000, 0xc040]); // 1, 2, -3 crossing a word
    put16(&mut ctx, y, 1024, &[0x4040, 0x3f80, 0x3f80]); // 3, 1, 1 at 2048-byte steps
    run(&mut ctx);
    assert_eq!((ctx.c, ctx.flag), (0x4000_0000, false));
    // The same lanes as FP16 are different numbers, and an ignored NaN lane stays ignored.
    ctx.mem.write(BASE + 32, 2 << 8, 8);
    run(&mut ctx);
    assert_eq!(ctx.c, 0x4070_0000); // 1.875*2.125 + 2*1.875 - 2.125*1.875
    ctx.step = 5;
    ctx.emulation_mode = EmulationMode::GenerateMemReads;
    floatdot_generate_mem_reads(&mut ctx);
    let data = &ctx.precompiled.input_data;
    assert_eq!(data.len(), 6 + 6);
    assert_eq!(data[6..9], [ctx.mem.read(BASE + 0x100, 8); 3]);
    assert_eq!(
        data[9..],
        [BASE + 0x1000, BASE + 0x1800, BASE + 0x2000].map(|a| ctx.mem.read(a, 8))
    );
    let mut consume = InstContext::new();
    consume.b = BASE;
    consume.step = 5;
    consume.emulation_mode = EmulationMode::ConsumeMemReads;
    consume.precompiled.input_data = data[..6].to_vec();
    floatdot_consume_mem_reads(&mut consume);
    assert_eq!((consume.data_ext_len, consume.c), (6, 0x4070_0000));
}

#[test]
fn sixteen_bit_descriptors_reject_reserved_encodings_and_wrapping() {
    for words in [
        [BASE | 1 << 32, BASE, 1, 0, 3 << 8],        // format 3
        [BASE + 1, BASE, 1, 0, 1 << 8],              // odd byte address
        [BASE | 1 << 48, BASE, 1, 0, 1 << 8],        // stride above 16 bits
        [BASE | 1 << 32, BASE, 1, 0, 0],             // FP32 with a stride
        [0xffff_fffe | 1 << 32, BASE, 2, 0, 2 << 8], // second element wraps
        [BASE, BASE, 1, 0, 1 << 8 | 1 << 5],         // reserved flag bit
    ] {
        let mut ctx = fixture(words);
        assert!(catch_unwind(AssertUnwindSafe(|| run(&mut ctx))).is_err(), "{words:x?}");
    }
    // Empty 16-bit calls preserve their state and read no vectors, even at the top address.
    let mut ctx = fixture([0xffff_fffe | 0xffff << 32, 0, 0, 0xff800001, 24 | 2 << 8]);
    run(&mut ctx);
    assert_eq!(ctx.c, 24 << 32 | 0xff800001);
}
