use crate::{zisk_ops::OpStats, EmulationMode, InstContext};
use zisk_precomp_helpers::{FloatDotCall, FloatDotCallHeader};

/// Validate the fixed replay header without accessing or allocating extended vector words.
/// This authenticates neither the claimed arithmetic result nor memory contents.
pub fn validate_floatdot_replay_header(
    a: u64,
    b: u64,
    step: u64,
    data: &[u64],
) -> Result<FloatDotCallHeader, &'static str> {
    if a != 0 {
        return Err("floatdot operand a is reserved");
    }
    FloatDotCallHeader::decode(b, step, data)
}

/// Build the fixed header and extended vector words for Rust proof replay.
pub fn floatdot_generate_mem_reads(ctx: &mut InstContext) {
    assert_eq!(ctx.emulation_mode, EmulationMode::GenerateMemReads);
    let payload = read_floatdot_call(ctx);
    ctx.c = payload[5];
    ctx.flag = false;
    ctx.data_ext_len = 0;
    ctx.precompiled.input_data = payload;
    ctx.precompiled.output_data.clear();
    ctx.precompiled.step = ctx.step;
}

/// Consume only six fixed words, as DMA does before dispatching extended data separately.
pub fn floatdot_consume_mem_reads(ctx: &mut InstContext) {
    assert_eq!(ctx.emulation_mode, EmulationMode::ConsumeMemReads);
    let header =
        validate_floatdot_replay_header(ctx.a, ctx.b, ctx.step, &ctx.precompiled.input_data)
            .expect("invalid FloatDot replay header");
    ctx.data_ext_len = header.data_ext_len();
    ctx.c = header.encode()[5];
    ctx.flag = false;
    ctx.precompiled.output_data.clear();
    ctx.precompiled.step = ctx.step;
}

fn aligned_range(address: u64, bytes: u64) {
    assert!(address < 1 << 32 && address & 7 == 0, "floatdot address must be aligned and 32-bit");
    assert!(bytes <= (1 << 32) - address, "floatdot memory range wraps");
}

fn read_floatdot_call(ctx: &InstContext) -> Vec<u64> {
    assert_eq!(ctx.a, 0, "floatdot operand a is reserved");
    aligned_range(ctx.b, 40);
    assert!(ctx.step < 1 << 36, "FloatDot VM step exceeds 36 bits");
    let mut words = [0; 6];
    for (i, word) in words[..5].iter_mut().enumerate() {
        *word = ctx.mem.read(ctx.b + u64::try_from(i).unwrap() * 8, 8);
    }
    let header = validate_floatdot_replay_header(ctx.a, ctx.b, ctx.step, &words)
        .expect("invalid FloatDot memory descriptor");
    let mut payload = Vec::with_capacity(6 + header.data_ext_len());
    payload.extend_from_slice(&words);
    let d = header.descriptor();
    for address in
        d.word_addresses(d.x, d.stride_x).into_iter().chain(d.word_addresses(d.y, d.stride_y))
    {
        payload.push(ctx.mem.read(u64::from(address), 8));
    }
    let result = FloatDotCall::decode(ctx.b, ctx.step, &payload)
        .expect("validated FloatDot memory payload")
        .reference_result();
    payload[5] = u64::from(result.bits) | (u64::from(result.flags) << 32);
    payload
}

/// Experimental Rust execution and replay; the matching FloatDot AIR/setup must be used.
pub fn opc_floatdot(ctx: &mut InstContext) {
    match ctx.emulation_mode {
        EmulationMode::Mem => {
            ctx.c = read_floatdot_call(ctx)[5];
            ctx.flag = false;
        }
        EmulationMode::GenerateMemReads => floatdot_generate_mem_reads(ctx),
        EmulationMode::ConsumeMemReads => floatdot_consume_mem_reads(ctx),
    }
}

pub fn op_floatdot(_a: u64, _b: u64) -> (u64, bool) {
    panic!("floatdot requires a validated memory descriptor");
}

pub fn ops_floatdot(_ctx: &InstContext, _stats: &mut dyn OpStats) {
    panic!("floatdot opcode cost profiling is unsupported");
}
