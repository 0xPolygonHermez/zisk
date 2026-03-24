use precompiles_helpers::DmaInfo;

use crate::{
    zisk_ops::{OpStats, ZiskOp},
    EmulationMode, InstContext,
};
#[inline(always)]
pub fn opc_dma_xmemset(ctx: &mut InstContext) {
    let dst = ctx.a;
    let count = ctx.b;
    let fill_byte = ctx.extended_arg as u8;

    match ctx.emulation_mode {
        EmulationMode::Mem => {
            ctx.mem.memset(dst, count, fill_byte);
        }
        EmulationMode::GenerateMemReads => {
            // In generate mode we need to populate precompiled.input_data with
            // information needed
            ctx.precompiled.input_data.clear();

            #[cfg(feature = "log_dma_ops")]
            println!(
                "opc_dma_memset 0x{dst:08X} 0x{fill_byte:02X} {count} GMR STEP:{} PC:0x{:08x}",
                ctx.step, ctx.pc
            );

            let encoded = DmaInfo::encode_memset(dst, count as usize, fill_byte);
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
                #[cfg(feature = "log_dma_ops")]
                println!(
                    "PRECOMPILED.MEMSET.INPUT_DATA: [{}] data_len:{data_len}",
                    ctx.precompiled
                        .input_data
                        .iter()
                        .map(|x| format!("0x{x:016X}"))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                assert_eq!(data_len as usize, DmaInfo::get_pre_writes(encoded));
                ctx.mem.memset(dst, count, fill_byte);
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
                "opc_dma_memset 0x{dst:08X} 0x{fill_byte:02X} {count} CMR STEP:{} DATA_EXT_LEN:{}",
                ctx.step,
                DmaInfo::get_data_size(encoded)
            );
            }
            ctx.data_ext_len = DmaInfo::get_pre_writes(encoded);
        }
    }
    ctx.c = dst;
    ctx.flag = false;
}

#[inline(always)]
pub fn op_dma_xmemset(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_dma_memset() is not implemented");
}

#[inline(always)]
pub fn ops_dma_xmemset(ctx: &InstContext, stats: &mut dyn OpStats) {
    let addr_a = ctx.a;
    let count = ctx.b;

    // pre, post, dma_align, dma_unalign
    if count == 0 {
        return;
    }

    let offset_a = addr_a & 0x07;
    let addr64_a = addr_a - offset_a;
    let pre_count = std::cmp::min((8 - offset_a) & 0x07, count);

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
        // add information about other machines to demostrate operation
        stats.add_extras(&[
            (ZiskOp::_DMA_PRE, (pre_count > 0) as usize),
            (ZiskOp::_DMA_POST, (post_count > 0) as usize),
            (ZiskOp::_DMA_64_ALIGNED, loop_count),
        ]);
    }
}
