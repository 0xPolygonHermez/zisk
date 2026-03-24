use precompiles_helpers::DmaInfo;

use crate::{
    zisk_ops::{OpStats, ZiskOp},
    EmulationMode, InstContext, FCALL_RESULT_MAX_SIZE,
};

fn read_from_input(ctx: &mut InstContext, dst: u64, count: u64) {
    // Check for consistency
    if count % 8 != 0 {
        panic!("opc_dma_inputcpy() called without invalid count {count}");
    }
    let count64 = count >> 3;
    if ctx.fcall.result_size == 0 {
        panic!("opc_dma_inputcpy() called with ctx.fcall.result_size==0");
    }
    if ctx.fcall.result_size as usize > FCALL_RESULT_MAX_SIZE {
        panic!(
            "opc_dma_inputcpy() called with ctx.fcall.result_size=={}>32",
            ctx.fcall.result_size
        );
    }
    if (ctx.fcall.result_got - 1 + count64) > ctx.fcall.result_size {
        panic!(
            "opc_dma_inputcpy() called with ctx.fcall.result_got({}) + {count64} >= ctx.fcall.result_size {}",
            ctx.fcall.result_got, ctx.fcall.result_size
        );
    }
    ctx.mem.memcpy_from_data(
        dst,
        count,
        &ctx.fcall.result,
        (ctx.fcall.result_got - 1) as usize * 8,
    );
    ctx.fcall.result_got += count64;
    if ctx.fcall.result_got > ctx.fcall.result_size {
        ctx.mem.free_input = 0;
    } else {
        ctx.mem.free_input = ctx.fcall.result[ctx.fcall.result_got as usize - 1];
    }
}

fn read_and_get_from_input(ctx: &mut InstContext, dst: u64, count: u64) -> Vec<u64> {
    // Check for consistency
    if count % 8 != 0 {
        panic!("opc_dma_inputcpy() called at 0x{:08x} without invalid count {count}", ctx.pc);
    }
    let count64 = count >> 3;
    if ctx.fcall.result_size == 0 {
        panic!("opc_dma_inputcpy() called at 0x{:08x} with ctx.fcall.result_size==0", ctx.pc);
    }
    if ctx.fcall.result_size as usize > FCALL_RESULT_MAX_SIZE {
        panic!(
            "opc_dma_inputcpy() called at 0x{:08x} with ctx.fcall.result_size=={}>32",
            ctx.pc, ctx.fcall.result_size
        );
    }
    if (ctx.fcall.result_got - 1 + count64) > ctx.fcall.result_size {
        panic!(
            "opc_dma_inputcpy() called at 0x{:08x} with ctx.fcall.result_got({}) + {count64} >= ctx.fcall.result_size {}",
            ctx.pc, ctx.fcall.result_got, ctx.fcall.result_size
        );
    }

    ctx.mem.memcpy_from_data(
        dst,
        count,
        &ctx.fcall.result,
        (ctx.fcall.result_got - 1) as usize * 8,
    );

    let offset = (dst & 0x07) as usize;
    let start_index = (ctx.fcall.result_got - 1) as usize;
    let mut _qwords_added = 0;
    let mut input_data = Vec::new();

    if offset == 0 {
        // Fast path: aligned, direct copy
        for i in 0..count64 as usize {
            input_data.push(ctx.fcall.result[start_index + i]);
            _qwords_added += 1;
        }
    } else {
        // Slow path: unaligned, need to shift and merge words
        // When unaligned, we need count64 + 1 output words
        let shift_bits = (offset * 8) as u32;
        let shift_bits_comp = 64 - shift_bits;

        // First word: padding zeros in lower bytes, first data bytes in upper bytes
        let first_word = ctx.fcall.result[start_index] << shift_bits;
        input_data.push(first_word);
        _qwords_added += 1;

        // Middle words: merge parts of consecutive data words
        for i in 0..(count64 as usize - 1) {
            let low_part = ctx.fcall.result[start_index + i] >> shift_bits_comp;
            let high_part = ctx.fcall.result[start_index + i + 1] << shift_bits;
            input_data.push(low_part | high_part);
            _qwords_added += 1;
        }

        // Last word: remaining bytes from last data word
        if count64 > 0 {
            let last_word = ctx.fcall.result[start_index + count64 as usize - 1] >> shift_bits_comp;
            input_data.push(last_word);
            _qwords_added += 1;
        }
    }

    ctx.fcall.result_got += count64;
    if ctx.fcall.result_got > ctx.fcall.result_size {
        ctx.mem.free_input = 0;
    } else {
        ctx.mem.free_input = ctx.fcall.result[ctx.fcall.result_got as usize - 1];
    }

    input_data
}

#[inline(always)]
pub fn opc_dma_inputcpy(ctx: &mut InstContext) {
    let dst: u64 = ctx.a;
    let count = ctx.b;

    match ctx.emulation_mode {
        EmulationMode::Mem => {
            read_from_input(ctx, dst, count);
        }
        EmulationMode::GenerateMemReads => {
            // In generate mode we need to populate precompiled.input_data with
            // information needed
            ctx.precompiled.input_data.clear();

            #[cfg(feature = "log_dma_ops")]
            println!("opc_dma_inputcpy 0x{dst:08X} {count} GMR STEP:{}", ctx.step);

            let encoded = DmaInfo::encode_inputcpy(dst, count as usize);
            ctx.precompiled.input_data.push(encoded);

            if count > 0 {
                // read first dst unaligned part for dma-pre
                let mut data_len = 0;
                let dst64 = dst & !0x07;
                // if dst64 != dst {
                if DmaInfo::get_pre_count(encoded) > 0 {
                    let pre_data = ctx.mem.read(dst64, 8);
                    data_len += 1;
                    ctx.precompiled.input_data.push(pre_data);
                }

                // read last dst unaligned part for dma-post
                let to_dst = dst + count - 1;
                // if to_dst & 0x07 != 0x07 {
                if DmaInfo::get_post_count(encoded) > 0 {
                    let post_data = ctx.mem.read(to_dst & !0x07, 8);
                    data_len += 1;
                    ctx.precompiled.input_data.push(post_data);
                }
                #[cfg(feature = "debug_dma")]
                println!(
                    "PRECOMPILED.INPUTCPY.INPUT_DATA: [{}] data_len:{data_len}",
                    ctx.precompiled
                        .input_data
                        .iter()
                        .map(|x| format!("0x{x:016X}"))
                        .collect::<Vec<_>>()
                        .join(",")
                );

                let input_data = read_and_get_from_input(ctx, dst, count);
                data_len += input_data.len();

                assert_eq!(data_len, DmaInfo::get_data_size(encoded));

                ctx.precompiled.input_data.extend(input_data);
            }
            ctx.precompiled.output_data.clear();
            ctx.precompiled.step = ctx.step;
        }
        EmulationMode::ConsumeMemReads => {
            let encoded = ctx.precompiled.input_data[0];
            let _count = DmaInfo::get_count(encoded);
            #[cfg(feature = "debug_dma")]
            println!(
                "opc_dma_inputcpy 0x{dst:08X} {count} CMR STEP:{} DATA_EXT_LEN:{}",
                ctx.step,
                DmaInfo::get_data_size(encoded)
            );
            ctx.data_ext_len = DmaInfo::get_data_size(encoded);
        }
    }
    ctx.c = dst;
    ctx.flag = false;
}

#[inline(always)]
pub fn op_dma_inputcpy(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_dma_inputcpy() is not implemented");
}

#[inline(always)]
pub fn ops_dma_inputcpy(ctx: &InstContext, stats: &mut dyn OpStats) {
    let addr_a = ctx.a;
    let count = ctx.b;

    // pre, post, dma_align, dma_unalign
    if count == 0 {
        return;
    }

    let offset_a = addr_a & 0x07;
    let addr64_a = addr_a - offset_a;
    let pre_count = (8 - offset_a) & 0x07;

    if pre_count > 0 {
        stats.mem_align_read(addr64_a, 1);
        stats.mem_align_write(addr64_a, 1);
    }

    let post_count = (count - pre_count) & 0x07;
    let addr64_a_end = (addr_a + count - 1) & !0x07;
    if post_count > 0 {
        stats.mem_align_read(addr64_a_end, 1);
        stats.mem_align_write(addr64_a_end, 1);
    }

    let loop_count = ((count - pre_count - post_count) >> 32) as usize;
    if loop_count == 0 {
        // with count < 8, there aren't 64-bits loops.
        stats.add_extras(&[
            (ZiskOp::_DMA_PRE, (pre_count > 0) as usize),
            (ZiskOp::_DMA_POST, (post_count > 0) as usize),
        ]);
    } else {
        // calculate the resources used by 64-bits loop.
        // count used are number of bytes read to demostrate memcmp(), usually count_eq + 1,
        // but if all bytes are equal count = count_eq, no need extra reads
        let first_loop_dst64 = (addr_a + pre_count) >> 3;

        stats.mem_align_write(first_loop_dst64, loop_count);
        stats.add_extras(&[
            (ZiskOp::_DMA_PRE, (pre_count > 0) as usize),
            (ZiskOp::_DMA_POST, (post_count > 0) as usize),
            (ZiskOp::_DMA_64_ALIGNED, loop_count),
        ]);
    }
}
