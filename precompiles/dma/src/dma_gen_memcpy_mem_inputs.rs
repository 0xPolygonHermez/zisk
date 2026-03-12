use precompiles_common::MemBusHelpers;
use precompiles_common::MemProcessor;
use precompiles_helpers::{DmaHelpers, DmaInfo};
use zisk_common::{A, B, DMA_ENCODED, OP, STEP};
use zisk_core::{zisk_ops::ZiskOp, EXTRA_PARAMS_ADDR};

pub fn generate_dma_memcpy_mem_inputs<P: MemProcessor>(
    data: &[u64],
    data_ext: &[u64],
    mem_processors: &mut P,
) {
    let op = data[OP] as u8;

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

    if op == ZiskOp::DMA_MEMCPY {
        MemBusHelpers::mem_aligned_read(
            EXTRA_PARAMS_ADDR as u32,
            main_step,
            DmaInfo::get_count(encoded) as u64,
            mem_processors,
        );
    }

    let pre_write_value = if pre_count > 0 {
        let pre_data_offset = DmaInfo::get_pre_data_offset(encoded);
        let read_value = data_ext[pre_data_offset];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load@pre 0x{src64:08X} S:{main_step} V:{read_value} (0x{read_value:016X})");
        MemBusHelpers::mem_aligned_read(src64, main_step, read_value, mem_processors);
        // pre-load of write address before unaligned write
        let pre_value = data_ext[DmaInfo::get_pre_write_offset(encoded)];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load@pre-p 0x{dst64:08X} S:{main_step} V:{pre_value} (0x{pre_value:016X})");

        MemBusHelpers::mem_aligned_read(dst64, main_step, pre_value, mem_processors);

        if DmaInfo::is_double_read_pre(encoded) {
            let second_read_value = data_ext[pre_data_offset + 1];
            #[cfg(feature = "debug_dma")]
            println!(
                "DMA: mem_aligned_load@pre2 0x{:08X} S:{main_step} V:{second_read_value} (0x{second_read_value:016X})",
                src64 + 8
            );
            MemBusHelpers::mem_aligned_read(
                src64 + 8,
                main_step,
                second_read_value,
                mem_processors,
            );
            Some(DmaHelpers::calculate_write_value(
                dst_offset,
                src_offset,
                pre_count,
                pre_value,
                &[read_value, second_read_value],
            ))
        } else {
            Some(DmaHelpers::calculate_write_value(
                dst_offset,
                src_offset,
                pre_count,
                pre_value,
                &[read_value],
            ))
        }
    } else {
        None
    };

    let post_count = DmaInfo::get_post_count(encoded) as u64;
    let loop_count = DmaInfo::get_loop_count(encoded);

    let post_write_value = if post_count > 0 {
        let src_offset = src & 0x07;

        let post_data_offset = DmaInfo::get_post_data_offset(encoded);
        let src64 = (src as u32 + pre_count as u32 + loop_count as u32 * 8) & !0x07;
        let dst64 = dst as u32 + pre_count as u32 + loop_count as u32 * 8;
        let read_value = data_ext[post_data_offset];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load@post 0x{src64:08X} S:{main_step} V:{read_value} (0x{read_value:016X})");
        MemBusHelpers::mem_aligned_read(src64, main_step, read_value, mem_processors);

        // pre-load of write address before unaligned write
        let pre_value = data_ext[DmaInfo::get_post_write_offset(encoded)];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load@post-p 0x{dst64:08X} S:{main_step} V:{pre_value} (0x{pre_value:016X})");

        MemBusHelpers::mem_aligned_read(dst64, main_step, pre_value, mem_processors);

        if DmaInfo::is_double_read_post(encoded) {
            let second_read_value = data_ext[post_data_offset + 1];
            #[cfg(feature = "debug_dma")]
            println!(
                "DMA: mem_aligned_load@post2 0x{:08X} S:{main_step} V:{second_read_value} (0x{second_read_value:016X})",
                src64 + 8
            );
            MemBusHelpers::mem_aligned_read(
                src64 + 8,
                main_step,
                second_read_value,
                mem_processors,
            );
            Some(DmaHelpers::calculate_write_value(
                0,                               // in post offset it's 0
                (src_offset + pre_count) & 0x07, // src_offset it's modified by pre, aligned/unaligned no change offset
                post_count,
                pre_value,
                &[read_value, second_read_value],
            ))
        } else {
            Some(DmaHelpers::calculate_write_value(
                0,                               // in post offset it's 0
                (src_offset + pre_count) & 0x07, // src_offset it's modified by pre, aligned/unaligned no change offset
                post_count,
                pre_value,
                &[read_value],
            ))
        }
    } else {
        None
    };

    // NOTE: loop it's done at end to do loop write after loop reads in same scope to avoid
    // recalculate values or extract them from scope.

    if loop_count > 0 {
        let loop_src = src as u32 + pre_count as u32;
        let dst64 = (dst as u32 + pre_count as u32) & !0x07;
        let src64 = loop_src & !0x07;
        let loop_data_offset = DmaInfo::get_loop_data_offset(encoded);
        let loop_data_count = DmaInfo::get_loop_count(encoded);
        let loop_src_data_end =
            loop_data_offset + loop_data_count + ((loop_src & 0x07) > 0) as usize;
        let values = &data_ext[loop_data_offset..loop_src_data_end];

        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_load_from_slice 0x{src64:08X} S:{main_step} V:{values:?}");
        MemBusHelpers::mem_aligned_read_from_slice(src64, main_step, values, mem_processors);

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

    if let Some(write_value) = pre_write_value {
        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_write@pre 0x{dst64:08X} S:{main_step} V:{write_value} (0x{write_value:016X})");

        MemBusHelpers::mem_aligned_write(dst64, main_step, write_value, mem_processors);
    }
    if let Some(write_value) = post_write_value {
        #[cfg(feature = "debug_dma")]
        println!("DMA: mem_aligned_write@post 0x{dst64:08X} S:{main_step} V:{write_value} (0x{write_value:016X})");
        let dst64 = dst as u32 + pre_count as u32 + loop_count as u32 * 8;
        MemBusHelpers::mem_aligned_write(dst64, main_step, write_value, mem_processors);
    }
}

pub fn skip_dma_memcpy_mem_inputs<P: MemProcessor>(data: &[u64], mem_processors: &mut P) -> bool {
    let dst = data[A];
    let src = data[B];

    let count = DmaInfo::get_count(data[DMA_ENCODED]) as u64;

    // calculate range for dst and src to verify if any of them are included
    // in the memcollector addresses.

    let dst64_from = dst as u32 & !0x07;
    let src64_from = src as u32 & !0x07;
    let dst64_to = (dst + count + 7) as u32 & !0x07;
    let src64_to = (src + count + 7) as u32 & !0x07;

    mem_processors.skip_addr(EXTRA_PARAMS_ADDR as u32)
        && mem_processors.skip_addr_range(dst64_from, dst64_to)
        && mem_processors.skip_addr_range(src64_from, src64_to)
}
