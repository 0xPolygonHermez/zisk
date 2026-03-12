use precompiles_common::MemBusHelpers;
use precompiles_common::MemProcessor;
use precompiles_helpers::DmaInfo;
use zisk_common::{A, DMA_ENCODED, STEP};

pub fn generate_dma_memset_mem_inputs<P: MemProcessor>(
    data: &[u64],
    data_ext: &[u64],
    mem_processors: &mut P,
) {
    let dst = data[A];
    let encoded = data[DMA_ENCODED];
    let dst64 = (dst & !0x07) as u32;
    let dst_offset = dst & 0x07;
    let step = data[STEP];
    let pre_count = DmaInfo::get_pre_count(encoded) as u64;
    let post_count = DmaInfo::get_post_count(encoded) as u64;

    debug_assert_eq!(
        (pre_count > 0) as usize + (post_count > 0) as usize,
        data_ext.len(),
        "[dma_memset] data length mismatch DATA:[{}] INFO={}",
        data.iter().map(|v| format!("0x{v:016X}")).collect::<Vec<String>>().join(", "),
        DmaInfo::to_string(encoded)
    );

    // the memset operation was simple, no unaligned loop, on aligned loop no need to read previous value
    // really only need read previous value if has pre o post

    if pre_count > 0 {
        #[cfg(feature = "debug_dma_gen_mem_inputs")]
        println!("[dma_memset] INPUT PRE DST:0x{dst64:08X} S:{step}");
        MemBusHelpers::mem_aligned_read(dst64, step, data_ext[0], mem_processors);
    }

    let loop_count = DmaInfo::get_loop_count(encoded);

    if post_count > 0 {
        let dst64_post = ((dst + pre_count) as usize + loop_count * 8) as u32;

        #[cfg(feature = "debug_dma_gen_mem_inputs")]
        println!("[dma_memset] INPUT POST DST:0x{dst64_post:08X} S:{step}");
        MemBusHelpers::mem_aligned_read(
            dst64_post,
            step,
            data_ext[(pre_count > 0) as usize],
            mem_processors,
        );
    }

    let fill_byte = DmaInfo::get_fill_byte(encoded) as u64;
    let fill_word = fill_byte
        | fill_byte << 8
        | fill_byte << 16
        | fill_byte << 24
        | fill_byte << 32
        | fill_byte << 40
        | fill_byte << 48
        | fill_byte << 56;

    if pre_count > 0 {
        #[cfg(feature = "debug_dma_gen_mem_inputs")]
        println!("[dma_memset] INPUT PRE WRITE DST:0x{dst64:08X} S:{step}");
        let mask = (0xFFFF_FFFF_FFFF_FFFFu64 >> (64 - pre_count * 8)) << (dst_offset * 8);
        let write_value = (fill_word & mask) | (data_ext[0] & !mask);
        MemBusHelpers::mem_aligned_write(dst64, step, write_value, mem_processors);
    }

    if loop_count > 0 {
        let dst64_loop = dst as u32 + pre_count as u32;
        #[cfg(feature = "debug_dma_gen_mem_inputs")]
        println!("[dma_memset] INPUT LOOP DST:0x{dst64_loop:08X} C:{loop_count} S:{step}");
        MemBusHelpers::mem_aligned_write_pattern(
            dst64_loop,
            step,
            fill_word,
            loop_count,
            mem_processors,
        );
    }
    if post_count > 0 {
        let dst64_post = ((dst + pre_count) as usize + loop_count * 8) as u32;
        #[cfg(feature = "debug_dma_gen_mem_inputs")]
        println!("[dma_memset] INPUT POST WRITE DST:0x{dst64_post:08X} S:{step}");
        let mask = 0xFFFF_FFFF_FFFF_FFFFu64 >> (64 - post_count * 8);
        let write_value = (fill_word & mask) | (data_ext[(pre_count > 0) as usize] & !mask);
        MemBusHelpers::mem_aligned_write(dst64_post, step, write_value, mem_processors);
    }
}

pub fn skip_dma_memset_mem_inputs<P: MemProcessor>(data: &[u64], mem_processors: &mut P) -> bool {
    let dst = data[A];
    let encoded = data[DMA_ENCODED];
    let dst64 = (dst & !0x07) as u32;
    let dst64_to = (dst + DmaInfo::get_count(encoded) as u64 - 1) as u32 & !0x07;

    #[cfg(feature = "debug_dma_gen_mem_inputs")]
    let step = data[STEP];
    #[cfg(feature = "debug_dma_gen_mem_inputs")]
    println!("[dma_memset] SKIP DST:[0x{dst64:08X}..=0x{dst64_to:08X}] S:{step}");

    mem_processors.skip_addr_range(dst64, dst64_to)
}
