use precompiles_common::MemBusHelpers;
use precompiles_common::MemProcessor;
use precompiles_helpers::DmaInfo;
use zisk_common::{A, DMA_ENCODED, STEP};

pub fn generate_dma_inputcpy_mem_inputs<P: MemProcessor>(
    data: &[u64],
    data_ext: &[u64],
    mem_processors: &mut P,
) {
    // inputcpy has same offset that dst, but when prepare data add zero-bytes before the input
    // to emulate aligned operation.

    let dst = data[A];
    let dst_offset = dst & 0x07;
    let encoded = data[DMA_ENCODED];
    let dst64 = (dst & !0x07) as u32;
    let main_step = data[STEP];
    let pre_count = DmaInfo::get_pre_count(encoded) as u64;

    // NOTE: for dual memories it's very important to keep the order of loads and stores because
    // stores happend after loads.

    let pre_value = if pre_count > 0 {
        let pre_data_offset = DmaInfo::get_pre_data_offset(encoded);

        // pre-load of write address before unaligned write
        let value_before_write = data_ext[DmaInfo::get_pre_write_offset(encoded)];
        MemBusHelpers::mem_aligned_read(dst64, main_step, value_before_write, mem_processors);

        // if src and dst have the same offset, no double read
        // TBO: calculate_write_value with same offset

        let dst_offset_bits = dst_offset * 8;
        let mask = 0xFFFF_FFFF_FFFF_FFFF << dst_offset_bits;
        Some((value_before_write & !mask) | (data_ext[pre_data_offset] & mask))
    } else {
        None
    };

    let post_count = DmaInfo::get_post_count(encoded) as u64;
    let loop_count = DmaInfo::get_loop_count(encoded);

    let loop_values = if loop_count > 0 {
        let loop_data_offset = DmaInfo::get_loop_data_offset(encoded);
        let loop_data_count = DmaInfo::get_loop_count(encoded);
        let loop_data_end = loop_data_offset + loop_data_count;

        Some(&data_ext[loop_data_offset..loop_data_end])
    } else {
        None
    };

    let post_value = if post_count > 0 {
        let post_data_offset = DmaInfo::get_post_data_offset(encoded);
        let dst64 = dst as u32 + pre_count as u32 + loop_count as u32 * 8;

        // pre-load of write address before unaligned write
        let value_before_write = data_ext[DmaInfo::get_post_write_offset(encoded)];
        MemBusHelpers::mem_aligned_read(dst64, main_step, value_before_write, mem_processors);

        let post_bits = post_count * 8;
        let mask = 0xFFFF_FFFF_FFFF_FFFF << post_bits;
        Some((value_before_write & mask) | (data_ext[post_data_offset] & !mask))
    } else {
        None
    };

    // Before writes, all reads should be done, to avoid issues with dual memory

    if let Some(pre_value) = pre_value {
        MemBusHelpers::mem_aligned_write(dst64, main_step, pre_value, mem_processors);
    }
    if let Some(loop_values) = loop_values {
        let dst64 = (dst as u32 + pre_count as u32) & !0x07;
        MemBusHelpers::mem_aligned_write_from_slice(dst64, main_step, loop_values, mem_processors);
    }
    if let Some(post_value) = post_value {
        let dst64 = dst as u32 + pre_count as u32 + loop_count as u32 * 8;
        MemBusHelpers::mem_aligned_write(dst64, main_step, post_value, mem_processors);
    }
}

pub fn skip_dma_inputcpy_mem_inputs<P: MemProcessor>(data: &[u64], mem_processors: &mut P) -> bool {
    let dst = data[A];

    let count = DmaInfo::get_count(data[DMA_ENCODED]) as u64;
    // calculate range for dst and src to verify if any of them are included
    // in the memcollector addresses.

    let dst64_from = dst as u32 & !0x07;
    let dst64_to = (dst + count + 7) as u32 & !0x07;

    if !mem_processors.skip_addr_range(dst64_from, dst64_to) {
        return false;
    }

    // If any mem_collector includes this addresses we could skip this precompiles
    // at mem input data generation.
    true
}
