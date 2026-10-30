use zisk_precomp_helpers::DmaInfo;

use crate::zisk_ops::OpStats;

/// Reports the 64-bit loop and the memory reads the proof issues for one DMA operation with a
/// source (`memcpy` or `memcmp`) of `dst` and `src` with the bus `encoded` value, so the occupancy
/// analyses see what the memory state machine sees. `kind` is the `DMA_LOOP_*` of the operation.
///
/// The head and the tail read the source word (two when the encoding says the bytes straddle a
/// word boundary) and preload the destination word; the loop reads one source word per word, plus
/// one more when the source is unaligned, since every word is then stitched from two. A `memcmp`
/// loop also reads the destination words, which the caller reports.
pub(crate) fn report_dma_src_proof_reads(
    kind: u8,
    dst: u64,
    src: u64,
    encoded: u64,
    stats: &mut dyn OpStats,
) {
    let pre_count = DmaInfo::get_pre_count(encoded) as u64;
    let loop_count = DmaInfo::get_loop_count(encoded);
    let post_count = DmaInfo::get_post_count(encoded) as u64;

    if pre_count > 0 {
        let double = DmaInfo::is_double_read_pre(encoded);
        stats.mem_proof_read(src & !0x07, 1 + double as usize);
        stats.mem_proof_read(dst & !0x07, 1);
    }

    if loop_count > 0 {
        let aligned = dst & 0x07 == src & 0x07;
        stats.dma_loop(kind, loop_count, aligned);
        let loop_src = src + pre_count;
        stats.mem_proof_read(loop_src & !0x07, loop_count + ((loop_src & 0x07) > 0) as usize);
    }

    if post_count > 0 {
        let double = DmaInfo::is_double_read_post(encoded);
        let offset = pre_count + loop_count as u64 * 8;
        stats.mem_proof_read((src + offset) & !0x07, 1 + double as usize);
        stats.mem_proof_read(dst + offset, 1);
    }
}
