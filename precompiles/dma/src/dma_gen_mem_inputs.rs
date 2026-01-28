use precompiles_common::MemBusHelpers;
use precompiles_common::MemProcessor;
use precompiles_helpers::{DmaHelpers, DmaInfo};
use zisk_common::{A, B, DMA_ENCODED, OP, OPERATION_PRECOMPILED_BUS_DATA_SIZE, STEP};
use zisk_core::{zisk_ops::ZiskOp, EXTRA_PARAMS};

pub fn generate_dma_mem_inputs<P: MemProcessor>(
    data: &[u64],
    data_ext: &[u64],
    _only_counters: bool,
    mem_processors: &mut P,
) {
    let dst = data[A];
    let src = data[B];
    let encoded = data[DMA_ENCODED];

    let dst64 = (dst & !0x07) as u32;
    let src64 = (src & !0x07) as u32;
    let main_step = data[STEP];
    let pre_count = DmaInfo::get_pre_count(encoded) as u64;
    let dst_offset = dst & 0x07;
    let src_offset = src & 0x07;
    let aligned = dst_offset == src_offset;

    // NOTE: for dual memories it's very important to keep the order of loads and stores because
    // stores happend after loads.

    MemBusHelpers::mem_aligned_load(
        EXTRA_PARAMS as u32,
        main_step,
        DmaInfo::get_count(encoded) as u64,
        mem_processors,
    );

    if pre_count > 0 {
        let pre_data_offset = DmaInfo::get_pre_data_offset(encoded);
        let read_value = data_ext[pre_data_offset];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load@pre 0x{src64:08X} S:{main_step} V:{read_value} (0x{read_value:016X})");

        MemBusHelpers::mem_aligned_load(src64, main_step, read_value, mem_processors);

        // pre-load of write address before unaligned write
        let pre_value = data_ext[DmaInfo::get_pre_write_offset(encoded)];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load@pre-p 0x{dst64:08X} S:{main_step} V:{pre_value} (0x{pre_value:016X})");

        MemBusHelpers::mem_aligned_load(dst64, main_step, pre_value, mem_processors);

        let write_value = if DmaInfo::is_double_read_pre(encoded) {
            let second_read_value = data_ext[pre_data_offset + 1];
            #[cfg(feature = "debug_dma")]
            println!(
                "DMA: mem_aligned_load@pre2 0x{:08X} S:{main_step} V:{second_read_value} (0x{second_read_value:016X})",
                src64 + 8
            );
            MemBusHelpers::mem_aligned_load(
                src64 + 8,
                main_step,
                second_read_value,
                mem_processors,
            );
            DmaHelpers::calculate_write_value(
                dst_offset,
                src_offset,
                pre_count,
                pre_value,
                &[read_value, second_read_value],
            )
        } else {
            DmaHelpers::calculate_write_value(
                dst_offset,
                src_offset,
                pre_count,
                pre_value,
                &[read_value],
            )
        };
        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_write@pre 0x{dst64:08X} S:{main_step} V:{write_value} (0x{write_value:016X})");

        MemBusHelpers::mem_aligned_write(dst64, main_step, write_value, mem_processors);
    }

    // this is part of words loop
    let post_count = DmaInfo::get_post_count(encoded) as u64;
    let loop_count = DmaInfo::get_loop_count(encoded);
    if loop_count > 0 {
        let loop_src = src as u32 + pre_count as u32;
        let dst64 = (dst as u32 + pre_count as u32) & !0x07;
        let src64 = loop_src & !0x07;
        let loop_data_offset = DmaInfo::get_loop_data_offset(encoded);
        let loop_data_count = DmaInfo::get_loop_count(encoded);
        let loop_src_data_end =
            loop_data_offset + loop_data_count + ((loop_src & 0x07) > 0) as usize;
        if data_ext.len() <= loop_data_offset || data_ext.len() < loop_src_data_end {
            println!("PRE-CRASH data_ext[{loop_data_offset}..{loop_src_data_end}] data_ext.len() = {} DATA={data:?} INFO{}", data_ext.len(), DmaInfo::to_string(encoded));
        }
        let values = &data_ext[loop_data_offset..loop_src_data_end];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load_from_slice 0x{src64:08X} S:{main_step} V:{values:?}");

        MemBusHelpers::mem_aligned_load_from_slice(src64, main_step, values, mem_processors);

        let src_offset = (src_offset + pre_count) & 0x07;
        if aligned {
            #[cfg(feature = "debug_dma")]
            println!("DMA: mem_aligned_write_from_slice 0x{dst64:08X} S:{main_step} V:{values:?}");
            MemBusHelpers::mem_aligned_write_from_slice(dst64, main_step, values, mem_processors);
        } else {
            #[cfg(feature = "debug_dma")]
            println!("DMA: mem_aligned_write_from_read_unaligned_slice 0x{dst64:08X} S:{main_step} V:{values:?}");
            MemBusHelpers::mem_aligned_write_from_read_unaligned_slice(
                dst64,
                main_step,
                src_offset as u8,
                values,
                mem_processors,
            );
        }
    }
    if post_count > 0 {
        let src_offset = src & 0x07;

        let post_data_offset = DmaInfo::get_post_data_offset(encoded);
        let src64 = (src as u32 + pre_count as u32 + loop_count as u32 * 8) & !0x07;
        let dst64 = dst as u32 + pre_count as u32 + loop_count as u32 * 8;
        let read_value = data_ext[post_data_offset];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load@post 0x{src64:08X} S:{main_step} V:{read_value} (0x{read_value:016X})");

        MemBusHelpers::mem_aligned_load(src64, main_step, read_value, mem_processors);

        // pre-load of write address before unaligned write
        let pre_value = data_ext[DmaInfo::get_post_write_offset(encoded)];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load@post-p 0x{dst64:08X} S:{main_step} V:{pre_value} (0x{pre_value:016X})");

        MemBusHelpers::mem_aligned_load(dst64, main_step, pre_value, mem_processors);

        let write_value = if DmaInfo::is_double_read_post(encoded) {
            let second_read_value = data_ext[post_data_offset + 1];
            #[cfg(feature = "debug_dma")]
            println!(
                "DMA: mem_aligned_load@post2 0x{:08X} S:{main_step} V:{second_read_value} (0x{second_read_value:016X})",
                src64 + 8
            );
            MemBusHelpers::mem_aligned_load(
                src64 + 8,
                main_step,
                second_read_value,
                mem_processors,
            );
            DmaHelpers::calculate_write_value(
                0,                               // in post offset it's 0
                (src_offset + pre_count) & 0x07, // src_offset it's modified by pre, aligned/unaligned no change offset
                post_count,
                pre_value,
                &[read_value, second_read_value],
            )
        } else {
            DmaHelpers::calculate_write_value(
                0,                               // in post offset it's 0
                (src_offset + pre_count) & 0x07, // src_offset it's modified by pre, aligned/unaligned no change offset
                post_count,
                pre_value,
                &[read_value],
            )
        };

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_write@post 0x{dst64:08X} S:{main_step} V:{write_value} (0x{write_value:016X})");
        MemBusHelpers::mem_aligned_write(dst64, main_step, write_value, mem_processors);
    }
}

pub fn skip_dma_mem_inputs<P: MemProcessor>(
    data: &[u64],
    _data_ext: &[u64],
    mem_processors: &mut P,
) -> bool {
    let dst = data[A];
    let src = data[B];
    let op = data[OP] as u8;

    // A memcmp operation has two parts, any of them could be empty.
    // - equal part, means that bytes of src and dst are the same (count_eq)
    // - different part, at maximum one byte, to obtain difference (count - count_eq)
    let count = DmaInfo::get_count(data[DMA_ENCODED]) as u64;
    let use_count = match op {
        ZiskOp::DMA_MEMCPY => count,
        ZiskOp::DMA_MEMCMP => std::cmp::min(
            count,
            data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + 1] + 1, // count_eq + 1 (different byte)
        ),
        _ => panic!("Invalid operation inside skip_dma_mem_inputs (op:{op})"),
    };
    // calculate range for dst and src to verify if any of them are included
    // in the memcollector addresses.

    let dst64_from = dst as u32 & !0x07;
    let src64_from = src as u32 & !0x07;
    let dst64_to = (dst + use_count + 7) as u32 & !0x07;
    let src64_to = (src + use_count + 7) as u32 & !0x07;

    if !mem_processors.skip_addr(EXTRA_PARAMS as u32) {
        return false;
    }

    if !mem_processors.skip_addr_range(dst64_from, dst64_to) {
        return false;
    }

    if !mem_processors.skip_addr_range(src64_from, src64_to) {
        return false;
    }

    // If any mem_collector includes this addresses we could skip this precompiles
    // at mem input data generation.
    true
}
