use precompiles_helpers::DmaInfo;

use crate::{
    zisk_ops::OpStats, EmulationMode, InstContext, DMA_64_ALIGNED_MEMCPY_COST,
    DMA_64_ALIGNED_MEMCPY_DIVISOR, DMA_PRE_POST_MEMCPY_COST, DMA_UNALIGNED_MEMCPY_COST,
    EXTRA_PARAMS_ADDR,
};

pub fn opc_dma_memcpy(ctx: &mut InstContext) {
    opc_dma_memcpys(ctx, false)
}
pub fn opc_dma_xmemcpy(ctx: &mut InstContext) {
    opc_dma_memcpys(ctx, true)
}
fn opc_dma_memcpys(ctx: &mut InstContext, extended: bool) {
    let dst = ctx.a;
    let src = ctx.b;

    match ctx.emulation_mode {
        EmulationMode::Mem => {
            let count =
                if extended { ctx.extended_arg as u64 } else { ctx.mem.read(EXTRA_PARAMS_ADDR, 8) };
            ctx.mem.memcpy(dst, src, count);
        }
        EmulationMode::GenerateMemReads => {
            // In generate mode we need to populate precompiled.input_data with
            // information needed
            let count =
                if extended { ctx.extended_arg as u64 } else { ctx.mem.read(EXTRA_PARAMS_ADDR, 8) };
            ctx.precompiled.input_data.clear();

            #[cfg(feature = "log_dma_ops")]
            println!("opc_dma_memcpy 0x{dst:08X} 0x{src:08X} {count} GMR STEP:{}", ctx.step);

            let encoded = DmaInfo::encode_memcpy(dst, src, count as usize);
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
                    // println!("ADDING_POST_DATA 0x{:08X} 0x{post_data:016X}", to_dst & !0x07);
                    ctx.precompiled.input_data.push(post_data);
                }

                // read all source 64-words
                let src64 = src & !0x07;
                let to_src64 = (src + count - 1) & !0x07;

                let src64_count = (to_src64 - src64 + 8) >> 3;
                ctx.mem.push_from_mem(&mut ctx.precompiled.input_data, src64, src64_count * 8);
                data_len += src64_count;
                #[cfg(feature = "debug_dma")]
                println!(
                    "PRECOMPILED.MEMCPY.INPUT_DATA: [{}] data_len:{data_len}",
                    ctx.precompiled
                        .input_data
                        .iter()
                        .map(|x| format!("0x{x:016X}"))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                assert_eq!(data_len as usize, DmaInfo::get_data_size(encoded));

                ctx.mem.memcpy(dst, src, count);
            }
            ctx.precompiled.output_data.clear();

            ctx.precompiled.step = ctx.step;
        }
        EmulationMode::ConsumeMemReads => {
            let encoded = ctx.precompiled.input_data[0];
            #[cfg(feature = "debug_dma")]
            {
                let count = DmaInfo::get_count(encoded);
                println!(
                    "opc_dma_memcpy 0x{dst:08X} 0x{src:08X} {count} CMR STEP:{} DATA_EXT_LEN:{}",
                    ctx.step,
                    DmaInfo::get_data_size(encoded)
                );
            }
            ctx.data_ext_len = DmaInfo::get_data_size(encoded);
        }
    }
    ctx.c = dst;
    ctx.flag = false;
}

/// Unimplemented.  Arith256 can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_dma_memcpy(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_dma_memcpy() is not implemented");
}
#[inline(always)]
pub fn op_dma_xmemcpy(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_dma_xmemcpy() is not implemented");
}

#[inline(always)]
pub fn ops_dma_memcpy(ctx: &InstContext, stats: &mut dyn OpStats) {
    ops_dma_memcpys(ctx, stats, false)
}
#[inline(always)]
pub fn ops_dma_xmemcpy(ctx: &InstContext, stats: &mut dyn OpStats) {
    ops_dma_memcpys(ctx, stats, true)
}
#[inline(always)]
fn ops_dma_memcpys(ctx: &InstContext, stats: &mut dyn OpStats, extended: bool) {
    let addr_a = ctx.a;
    let addr_b = ctx.b;
    let count = if extended { ctx.extended_arg as u64 } else { ctx.mem.read(EXTRA_PARAMS_ADDR, 8) };
    // pre, post, dma_align, dma_unalign
    if !extended {
        stats.mem_align_read(EXTRA_PARAMS_ADDR, 1);
    }

    if count == 0 {
        return;
    }

    let offset_a = addr_a & 0x07;
    let offset_b = addr_b & 0x07;
    let addr64_a = addr_a - offset_a;
    let addr64_b = addr_b - offset_b;
    let pre_count = std::cmp::min((8 - offset_a) & 0x07, count);

    if pre_count > 0 {
        stats.mem_align_read(addr64_a, 1);
        stats.mem_align_read(addr64_b, 1 + ((offset_b + pre_count) > 8) as usize);
        stats.mem_align_write(addr64_a, 1);
    }

    let post_count = (count - pre_count) & 0x07;
    let remain_b = (16 - offset_a - pre_count) & 0x07;
    let addr64_a_end = (addr_a + count - 1) & !0x07;
    let addr64_b_end = (addr_b + count - 1) & !0x07;
    if post_count > 0 {
        let extra_b = (remain_b < post_count) as u64;
        stats.mem_align_read(addr64_a_end, 1);
        stats.mem_align_read(addr64_b_end - extra_b * 8, 1 + extra_b as usize);
        stats.mem_align_write(addr64_a_end, 1);
    }

    let loop_count = ((count - pre_count - post_count) >> 32) as usize;
    let variable_cost =
        DMA_PRE_POST_MEMCPY_COST * ((pre_count > 0) as u64 + (post_count > 0) as u64);

    if loop_count == 0 {
        // with count < 8, there aren't 64-bits loops.
        stats.set_variable_cost(variable_cost);
    } else {
        // calculate the resources used by 64-bits loop.
        // count used are number of bytes read to demostrate memcmp(), usually count_eq + 1,
        // but if all bytes are equal count = count_eq, no need extra reads
        let first_loop_dst64 = (addr_a + pre_count) >> 3;
        let first_loop_src64 = (addr_b + pre_count) >> 3;

        // same alignment
        if addr_a & 0x07 == addr_b & 0x07 {
            stats.mem_align_read(first_loop_src64, loop_count);
            stats.mem_align_write(first_loop_dst64, loop_count);
            // add information about other machines to demostrate operation
            stats.set_variable_cost(
                variable_cost
                    + (loop_count as u64).div_ceil(DMA_64_ALIGNED_MEMCPY_DIVISOR)
                        * DMA_64_ALIGNED_MEMCPY_COST,
            );
        } else {
            stats.mem_align_read(first_loop_src64, loop_count + 1);
            stats.mem_align_write(first_loop_dst64, loop_count);
            // add information about other machines to demostrate operation
            stats.set_variable_cost(
                variable_cost + (loop_count as u64 + 1) * DMA_UNALIGNED_MEMCPY_COST,
            );
        }
    }
}
