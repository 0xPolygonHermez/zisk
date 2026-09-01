//! Semantics of the `mt` DMA family and of the `execute_advice` hint that feeds it, exercised
//! straight through `InstContext` (which is how the emulator calls them).

use crate::operations::{
    opc_dma_mtcmp, opc_dma_mtcpy, opc_dma_xmtcmp, opc_dma_xmtcpy, opc_execute_advice,
};
use crate::ops_core_context::opc_flag;
use crate::{
    EmulationMode, InstContext, EXTRA_PARAMS_ADDR, EXTRA_PARAMS_TEMPORAL_REF_ADDR, RAM_ADDR,
    RAM_SIZE,
};
use zisk_definitions::TEMPORAL_REF_REQUEST_TAG;

/// Two 8-byte-aligned scratch buffers inside the write section, far enough apart not to overlap
const DST: u64 = RAM_ADDR + 0x1000;
const SRC: u64 = RAM_ADDR + 0x2000;

fn context() -> InstContext {
    let mut ctx = InstContext::new();
    ctx.mem.add_write_section(RAM_ADDR, RAM_SIZE);
    ctx.emulation_mode = EmulationMode::Mem;
    ctx.step = 100;
    ctx
}

fn write(ctx: &mut InstContext, addr: u64, bytes: &[u8]) {
    for (i, byte) in bytes.iter().enumerate() {
        ctx.mem.write(addr + i as u64, *byte as u64, 1);
    }
}

fn read(ctx: &InstContext, addr: u64, count: u64) -> Vec<u8> {
    (0..count).map(|i| ctx.mem.read(addr + i, 1) as u8).collect()
}

/// Requests a temporal reference the way the transpiled `csrrs rd, TEMPORAL_REF, x0` does, and
/// returns it
fn request_temporal_ref(ctx: &mut InstContext) -> u64 {
    ctx.a = 0;
    ctx.b = TEMPORAL_REF_REQUEST_TAG;
    opc_flag(ctx);
    ctx.step += 1;
    ctx.c
}

/// Runs the `execute_advice` hint over `[addr, addr + count)`
fn advise(ctx: &mut InstContext, addr: u64, count: u64) {
    ctx.a = addr;
    ctx.b = count;
    opc_execute_advice(ctx);
    ctx.step += 1;
}

#[test]
fn flag_returns_the_step_and_only_a_tagged_flag_is_a_request() {
    let mut ctx = context();
    ctx.step = 42;

    // An ordinary hint `addi x0, rs1, imm`: c is the step, but no request is recorded
    ctx.a = 0x1234;
    ctx.b = 7;
    opc_flag(&mut ctx);
    assert_eq!(ctx.c, 42);
    assert!(ctx.flag);
    assert_eq!(ctx.temporal_ref, 0);

    // A tagged flag is a request
    ctx.step = 43;
    ctx.a = 0;
    ctx.b = TEMPORAL_REF_REQUEST_TAG;
    opc_flag(&mut ctx);
    assert_eq!(ctx.c, 43);
    assert_eq!(ctx.temporal_ref, 43);
}

#[test]
fn mtcpy_reads_the_source_as_it_was_at_the_temporal_reference() {
    let mut ctx = context();
    let original: Vec<u8> = (0u8..24).collect();
    write(&mut ctx, SRC, &original);

    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 24);

    // The source is overwritten after being advised
    write(&mut ctx, SRC, &[0xFF; 24]);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 24, 8);
    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.a = DST;
    ctx.b = SRC;
    opc_dma_mtcpy(&mut ctx);

    assert_eq!(read(&ctx, DST, 24), original);
    assert_eq!(ctx.c, DST);
    assert!(!ctx.flag);
}

#[test]
fn xmtcpy_takes_its_count_from_the_extended_argument() {
    let mut ctx = context();
    let original: Vec<u8> = (0u8..16).collect();
    write(&mut ctx, SRC, &original);

    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 16);
    write(&mut ctx, SRC, &[0xFF; 16]);

    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.a = DST;
    ctx.b = SRC;
    ctx.extended_arg = 16;
    opc_dma_xmtcpy(&mut ctx);

    assert_eq!(read(&ctx, DST, 16), original);
}

#[test]
fn mtcpy_handles_unaligned_ranges() {
    let mut ctx = context();
    let original: Vec<u8> = (1u8..=13).collect();
    write(&mut ctx, SRC + 3, &original);

    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC + 3, 13);
    write(&mut ctx, SRC + 3, &[0xFF; 13]);

    // Bytes around the unaligned destination range must survive untouched
    write(&mut ctx, DST, &[0xAA; 24]);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 13, 8);
    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.a = DST + 5;
    ctx.b = SRC + 3;
    opc_dma_mtcpy(&mut ctx);

    assert_eq!(read(&ctx, DST + 5, 13), original);
    assert_eq!(read(&ctx, DST, 5), vec![0xAA; 5]);
    assert_eq!(read(&ctx, DST + 18, 6), vec![0xAA; 6]);
}

#[test]
fn mtcmp_compares_against_the_snapshot() {
    let mut ctx = context();
    let original: Vec<u8> = (0u8..16).collect();
    write(&mut ctx, SRC, &original);
    write(&mut ctx, DST, &original);

    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 16);
    write(&mut ctx, SRC, &[0xFF; 16]);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 16, 8);
    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.a = DST;
    ctx.b = SRC;
    opc_dma_mtcmp(&mut ctx);

    // Equal to the snapshot, even though it differs from the current source
    assert_eq!(ctx.c, 0);
    assert_eq!(ctx.stats_hint, 16);
}

#[test]
fn mtcmp_reports_the_first_difference() {
    let mut ctx = context();
    let original: Vec<u8> = (0u8..16).collect();
    write(&mut ctx, SRC, &original);
    write(&mut ctx, DST, &original);
    // dst[4] becomes larger than the snapshot's byte
    write(&mut ctx, DST + 4, &[10]);

    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 16);
    write(&mut ctx, SRC, &[0xFF; 16]);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 16, 8);
    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.a = DST;
    ctx.b = SRC;
    opc_dma_mtcmp(&mut ctx);

    assert_eq!(ctx.c, 10 - 4);
    assert_eq!(ctx.stats_hint, 5);
}

#[test]
fn xmtcmp_takes_its_count_from_the_extended_argument() {
    let mut ctx = context();
    write(&mut ctx, SRC, &[7u8; 8]);
    write(&mut ctx, DST, &[7u8; 8]);

    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 8);
    write(&mut ctx, SRC, &[0u8; 8]);

    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.a = DST;
    ctx.b = SRC;
    ctx.extended_arg = 8;
    opc_dma_xmtcmp(&mut ctx);

    assert_eq!(ctx.c, 0);
}

#[test]
fn several_temporal_references_stay_independent() {
    let mut ctx = context();

    write(&mut ctx, SRC, &[1u8; 8]);
    let first = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 8);

    write(&mut ctx, SRC, &[2u8; 8]);
    let second = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 8);

    write(&mut ctx, SRC, &[3u8; 8]);

    assert_ne!(first, second);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 8, 8);
    for (tref, expected) in [(first, 1u8), (second, 2u8)] {
        ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
        ctx.a = DST;
        ctx.b = SRC;
        opc_dma_mtcpy(&mut ctx);
        assert_eq!(read(&ctx, DST, 8), vec![expected; 8]);
    }
}

#[test]
fn advice_can_cover_several_regions_of_one_temporal_reference() {
    let mut ctx = context();
    write(&mut ctx, SRC, &[1u8; 8]);
    write(&mut ctx, SRC + 0x100, &[2u8; 8]);

    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 8);
    advise(&mut ctx, SRC + 0x100, 8);

    write(&mut ctx, SRC, &[9u8; 8]);
    write(&mut ctx, SRC + 0x100, &[9u8; 8]);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 8, 8);
    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);

    ctx.a = DST;
    ctx.b = SRC;
    opc_dma_mtcpy(&mut ctx);
    assert_eq!(read(&ctx, DST, 8), vec![1u8; 8]);

    ctx.a = DST;
    ctx.b = SRC + 0x100;
    opc_dma_mtcpy(&mut ctx);
    assert_eq!(read(&ctx, DST, 8), vec![2u8; 8]);
}

#[test]
#[should_panic(expected = "cannot serve")]
fn mtcpy_panics_when_the_source_was_never_advised() {
    let mut ctx = context();
    write(&mut ctx, SRC, &[1u8; 8]);
    let tref = request_temporal_ref(&mut ctx);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 8, 8);
    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.a = DST;
    ctx.b = SRC;
    opc_dma_mtcpy(&mut ctx);
}

#[test]
#[should_panic(expected = "cannot serve")]
fn mtcpy_panics_when_the_advised_range_is_too_narrow() {
    let mut ctx = context();
    write(&mut ctx, SRC, &[1u8; 32]);
    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 8);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 32, 8);
    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.a = DST;
    ctx.b = SRC;
    opc_dma_mtcpy(&mut ctx);
}

#[test]
fn generate_mem_reads_records_the_snapshot_words() {
    let mut ctx = context();
    let original: Vec<u8> = (0u8..16).collect();
    write(&mut ctx, SRC, &original);

    let tref = request_temporal_ref(&mut ctx);
    advise(&mut ctx, SRC, 16);
    write(&mut ctx, SRC, &[0xFF; 16]);

    ctx.mem.write(EXTRA_PARAMS_ADDR, 16, 8);
    ctx.mem.write(EXTRA_PARAMS_TEMPORAL_REF_ADDR, tref, 8);
    ctx.emulation_mode = EmulationMode::GenerateMemReads;
    ctx.a = DST;
    ctx.b = SRC;
    opc_dma_mtcpy(&mut ctx);

    // header word, then the two aligned source words as they were at the temporal reference
    assert_eq!(ctx.precompiled.input_data.len(), 3);
    assert_eq!(
        ctx.precompiled.input_data[1],
        u64::from_le_bytes(original[0..8].try_into().unwrap())
    );
    assert_eq!(
        ctx.precompiled.input_data[2],
        u64::from_le_bytes(original[8..16].try_into().unwrap())
    );
    assert_eq!(read(&ctx, DST, 16), original);
}
