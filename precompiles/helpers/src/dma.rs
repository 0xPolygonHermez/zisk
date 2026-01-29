// use static_assertions::const_assert;
// const_assert!(CHUNK_MEM_STEP_BITS <= 24);

pub struct DmaHelpers {}

pub struct DmaValues {
    pub dst64: u64,
    pub src64: u64,
    pub src_offset: u64,
    pub dst_offset: u64,
    pub pre_count: u64,
    pub post_count: u64,
    pub memcpy_count: u64,
    pub src64_inc_by_pre: u64,
    pub src_offset_after_pre: u64,
}

const FAST_ENCODE_TABLE_SIZE: usize = 8 * 8 * 16;
const FAST_ENCODE_TABLE: [u64; FAST_ENCODE_TABLE_SIZE] = generate_fast_encode_table();

const fn generate_fast_encode_table() -> [u64; FAST_ENCODE_TABLE_SIZE] {
    let mut table = [0u64; FAST_ENCODE_TABLE_SIZE];
    // fill table
    let mut dst_offset: u64 = 0;
    while dst_offset < 8 {
        let base_index = dst_offset << 7;
        let mut src_offset: u64 = 0;
        while src_offset < 8 {
            let index = (base_index + (src_offset << 4)) as usize;
            let mut count: usize = 0;
            while count < 16 {
                let value = DmaInfo::encode_memcpy(dst_offset, src_offset, count);
                let loop_count = DmaInfo::get_loop_count(value) as u64;
                // The table is create to add directly de loop count and after all values
                // are correct, for this reason substract de count, because we need diference
                // between loop_count (shifted 32) and count (shifted 29)
                table[index + count] = ((value & 0x0000_0000_FFFF_FFFF) + (loop_count << 32))
                    .wrapping_sub((count as u64) << 29);
                count += 1;
            }
            src_offset += 1;
        }
        dst_offset += 1;
    }
    table
}

pub struct DmaInfo {}

impl DmaInfo {
    #[inline(always)]
    pub fn to_string(encoded: u64) -> String {
        format!("loop_count: {}|pre_writes: {}|dst_offset: {}|src_offset: {}|pre_count: {}|post_count: {}|extra_src_reads: {}", 
        Self::get_loop_count(encoded),
        Self::get_pre_writes(encoded),
        Self::get_dst_offset(encoded),
        Self::get_src_offset(encoded),
        Self::get_pre_count(encoded),
        Self::get_post_count(encoded),
        Self::get_extra_src_reads(encoded))
    }
    #[inline(always)]
    pub const fn fast_encode_memcpy(dst: u64, src: u64, count: usize) -> u64 {
        let table_count = if count >= 16 { count & 0x07 | 0x08 } else { count };
        FAST_ENCODE_TABLE[(((dst & 0x07) << 7) + ((src & 0x07) << 4)) as usize + table_count]
            .wrapping_add((count as u64) << 29)
    }

    pub const fn encode_memcpy(dst: u64, src: u64, count: usize) -> u64 {
        //                    #bits     bits
        // pre_count:  0-7        3     0-2
        // post_count: 0-7        3     3-5
        // pre_writes: 0,1,2      2     6-7
        // dst_offset: 0-7        3     8-10
        // src_offset: 0-7        3     11-13
        // double_src_pre: 0,1    1     14
        // double_src_post: 0,1   1     15
        // extra_src_reads: 0-3   2     16-17
        // src64_inc_by_pre:      1     18
        // unaligned_dst_src:     1     20
        // loop_count            32     32-63

        let dst_offset = dst & 0x07;
        let src_offset = src & 0x07;

        let count = count as u64;
        let (pre_count, loop_count, post_count) = if dst_offset > 0 {
            let _pre_count = 8 - dst_offset;
            if _pre_count >= count {
                (count, 0, 0)
            } else {
                let pending = count - _pre_count;
                (_pre_count, pending >> 3, pending & 0x07)
            }
        } else {
            (0, count >> 3, count & 0x07)
        };
        let pre_writes = (pre_count > 0) as u64 + (post_count > 0) as u64;
        // let to_src_offset = (src + count - 1) & 0x07;
        let src_offset_pos = (src_offset + pre_count) & 0x07;
        let double_src_post = (src_offset_pos + post_count) > 8;
        let double_src_pre = (src_offset + pre_count) > 8;
        let extra_src_reads =
            if count == 0 { 0 } else { (((src + count - 1) >> 3) - (src >> 3) + 1) - loop_count };

        let src64_inc_by_pre = (pre_count > 0 && (src_offset + pre_count) >= 8) as u64;
        let unaligned_dst_src = (src_offset != dst_offset) as u64;

        pre_count
            | (post_count << 3)
            | (pre_writes << 6)
            | (dst_offset << 8)
            | (src_offset << 11)
            | ((double_src_pre as u64) << 14)
            | ((double_src_post as u64) << 15)
            | (extra_src_reads << 16)
            | (src64_inc_by_pre << 18)
            | (unaligned_dst_src << 19)
            | (pre_count << 29) // optimization to read loop_count * 8 + pre_count
            | (loop_count << 32)
    }

    pub const fn get_extra_src_reads(encoded: u64) -> usize {
        (encoded as usize) >> 16 & 0x03
    }
    pub const fn get_count(encoded: u64) -> usize {
        Self::get_loop_count(encoded) * 8
            + Self::get_pre_count(encoded)
            + Self::get_post_count(encoded)
    }
    pub const fn get_dst_offset(encoded: u64) -> usize {
        (encoded as usize >> 8) & 0x07
    }

    pub const fn get_src_offset(encoded: u64) -> usize {
        (encoded as usize >> 11) & 0x07
    }

    pub const fn get_loop_count(encoded: u64) -> usize {
        (encoded >> 32) as usize
    }

    pub const fn get_pre_writes(encoded: u64) -> usize {
        (encoded as usize >> 6) & 0x03
    }

    pub const fn is_double_read_pre(encoded: u64) -> bool {
        encoded & (1 << 14) != 0
    }

    pub const fn is_double_read_post(encoded: u64) -> bool {
        encoded & (1 << 15) != 0
    }

    pub const fn get_pre_count(encoded: u64) -> usize {
        (encoded as usize) & 0x07
    }

    pub const fn get_post_count(encoded: u64) -> usize {
        (encoded as usize >> 3) & 0x07
    }

    pub const fn get_pre(encoded: u64) -> u8 {
        (Self::get_pre_count(encoded) > 0) as u8 + Self::is_double_read_pre(encoded) as u8
    }

    pub const fn get_post(encoded: u64) -> u8 {
        (Self::get_post_count(encoded) > 0) as u8 + Self::is_double_read_post(encoded) as u8
    }

    pub const fn get_src64_inc_by_pre(encoded: u64) -> usize {
        (encoded as usize >> 18) & 0x01
    }

    pub const fn get_loop_data_offset(encoded: u64) -> usize {
        let pre_count = Self::get_pre_count(encoded);
        Self::get_pre_writes(encoded)
            + (pre_count > 0 && (Self::get_src_offset(encoded) + pre_count) >= 8) as usize
    }

    pub const fn get_loop_src_offset(encoded: u64) -> u8 {
        (Self::get_src_offset(encoded) + Self::get_pre_count(encoded)) as u8 & 0x07
    }

    pub const fn get_src_size(encoded: u64) -> usize {
        Self::get_loop_count(encoded) + Self::get_extra_src_reads(encoded)
    }
    pub const fn get_data_size(encoded: u64) -> usize {
        Self::get_pre_writes(encoded) + Self::get_src_size(encoded)
    }
    pub const fn get_post_data_offset(encoded: u64) -> usize {
        Self::get_pre_writes(encoded) + Self::get_src_size(encoded)
            - (Self::is_double_read_post(encoded) as usize + 1)
    }
    pub const fn get_pre_write_offset(_encoded: u64) -> usize {
        0
    }
    pub const fn get_post_write_offset(encoded: u64) -> usize {
        (Self::get_pre_count(encoded) != 0) as usize
    }
    pub const fn get_pre_data_offset(encoded: u64) -> usize {
        Self::get_pre_writes(encoded)
    }
}

impl DmaHelpers {
    pub fn calculate_write_value(
        dst_offset: u64,
        src_offset: u64,
        count: u64,
        pre_value: u64,
        src_values: &[u64],
    ) -> u64 {
        let write_mask =
            (0xFFFF_FFFF_FFFF_FFFF << ((8 - count) * 8)) >> ((8 - dst_offset - count) * 8);
        let value = if dst_offset <= src_offset {
            (src_values[0] >> ((src_offset - dst_offset) * 8))
                | if dst_offset == src_offset {
                    0
                } else if (src_offset + count) > 8 {
                    if src_values.len() < 2 {
                        panic!("ERROR src_values: {:?} dst_offset: {dst_offset} src_offset: {src_offset} count: {count}", src_values);
                    }
                    src_values[1] << ((8 - src_offset + dst_offset) * 8)
                } else {
                    0
                }
        } else if dst_offset > src_offset {
            src_values[0] << ((dst_offset - src_offset) * 8)
        } else {
            // dst_offset = src_offset
            src_values[0]
        };
        #[cfg(feature = "debug_dma")]
        println!(
            "WRITE_MASK 0x{write_mask:016X} VALUE 0x{value:016X} SRC_VALUES 0x{:016X},0x{:016X} PRE_VALUE:{pre_value:016X} DST_OFFSET:{dst_offset} SRC_OFFSET:{src_offset} COUNT:{count}",
            src_values[0], if src_values.len() > 1 { src_values[1] } else { 0 }
        );
        (pre_value & !write_mask) | (value & write_mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to compute expected value using byte-by-byte copy
    fn expected_write_value(
        dst_offset: u64,
        src_offset: u64,
        count: u64,
        pre_value: u64,
        src_values: &[u64],
    ) -> u64 {
        // Convert pre_value to bytes (big-endian layout as used in the function)
        let mut result_bytes = pre_value.to_le_bytes();

        // Convert src_values to a contiguous byte array (big-endian)
        let mut src_bytes = Vec::new();
        for &val in src_values {
            src_bytes.extend_from_slice(&val.to_le_bytes());
        }

        // Copy count bytes from src_bytes[src_offset..] to result_bytes[dst_offset..]
        for i in 0..count as usize {
            result_bytes[dst_offset as usize + i] = src_bytes[src_offset as usize + i];
        }

        u64::from_le_bytes(result_bytes)
    }

    #[test]
    fn test_calculate_write_value_all_combinations() {
        // Test patterns for src_values
        let src0: u64 = 0x0102030405060708;
        let src1: u64 = 0x1112131415161718;

        // Test pattern for pre_value
        let pre_value: u64 = 0xAABBCCDDEEFF0011;

        // Iterate over all dst_offset values (0..8)
        for dst_offset in 0..8 {
            // For each dst_offset, count can be 1 to (8 - dst_offset)
            for count in 1..=(8 - dst_offset) {
                // For each valid (dst_offset, count), test all src_offset values
                // src_offset can be 0..8, but we need to ensure we have enough src data
                for src_offset in 0..8 {
                    // Determine if we need one or two src values
                    // We need two src values if (src_offset + count) > 8
                    let needs_two_src = (src_offset + count) > 8;
                    let src_values: Vec<u64> = if needs_two_src {
                        vec![src0, src1]
                    } else {
                        vec![src0, 0] // Always provide both for safety
                    };

                    let result = DmaHelpers::calculate_write_value(
                        dst_offset,
                        src_offset,
                        count,
                        pre_value,
                        &src_values,
                    );

                    let expected =
                        expected_write_value(dst_offset, src_offset, count, pre_value, &src_values);

                    assert_eq!(
                        result, expected,
                        "Failed for dst_offset={}, src_offset={}, count={}\n\
                         pre_value:  0x{:016X}\n\
                         src[0]:     0x{:016X}\n\
                         src[1]:     0x{:016X}\n\
                         expected:   0x{:016X}\n\
                         got:        0x{:016X}",
                        dst_offset, src_offset, count, pre_value, src0, src1, expected, result
                    );
                }
            }
        }
    }

    #[test]
    fn test_calculate_write_value_edge_cases() {
        let src0: u64 = 0x0102030405060708;
        let src1: u64 = 0x1112131415161718;
        let pre_value: u64 = 0xAABBCCDDEEFF0011;

        // Test case: dst_offset=0, count=8 (full overwrite)
        let result = DmaHelpers::calculate_write_value(0, 0, 8, pre_value, &[src0, src1]);
        assert_eq!(result, src0, "Full overwrite with aligned offsets failed");

        // Test case: dst_offset=0, count=1 (single byte at start)
        let result = DmaHelpers::calculate_write_value(0, 0, 1, pre_value, &[src0, src1]);
        let expected = 0xAABBCCDDEEFF0008u64;
        assert_eq!(result, expected, "Single byte at start failed");

        // Test case: dst_offset=7, count=1 (single byte at end)
        let result = DmaHelpers::calculate_write_value(7, 0, 1, pre_value, &[src0, src1]);
        let expected = 0x08BBCCDDEEFF0011;
        assert_eq!(result, expected, "Single byte at end failed");

        // Test case: src spans two values (src_offset=7, count=2)
        let result = DmaHelpers::calculate_write_value(0, 7, 2, pre_value, &[src0, src1]);
        let expected = 0xAABBCCDDEEFF1801;
        assert_eq!(result, expected, "Src spanning two values failed");
    }

    #[test]
    fn test_calculate_write_value_zero_patterns() {
        let src0: u64 = 0x0000000000000000;
        let src1: u64 = 0x0000000000000000;
        let pre_value: u64 = 0xFFFFFFFFFFFFFFFF;

        // Writing zeros should clear the appropriate bytes
        for dst_offset in 0..8 {
            for count in 1..=(8 - dst_offset) {
                let result = DmaHelpers::calculate_write_value(
                    dst_offset,
                    0,
                    count,
                    pre_value,
                    &[src0, src1],
                );
                let expected = expected_write_value(dst_offset, 0, count, pre_value, &[src0, src1]);
                assert_eq!(
                    result, expected,
                    "Zero pattern failed for dst_offset={}, count={}",
                    dst_offset, count
                );
            }
        }
    }

    #[test]
    fn test_calculate_write_value_ff_patterns() {
        let src0: u64 = 0xFFFFFFFFFFFFFFFF;
        let src1: u64 = 0xFFFFFFFFFFFFFFFF;
        let pre_value: u64 = 0x0000000000000000;

        // Writing 0xFF should set the appropriate bytes
        for dst_offset in 0..8 {
            for count in 1..=(8 - dst_offset) {
                let result = DmaHelpers::calculate_write_value(
                    dst_offset,
                    0,
                    count,
                    pre_value,
                    &[src0, src1],
                );
                let expected = expected_write_value(dst_offset, 0, count, pre_value, &[src0, src1]);
                assert_eq!(
                    result, expected,
                    "FF pattern failed for dst_offset={}, count={}",
                    dst_offset, count
                );
            }
        }
    }

    /// Byte-based implementation for comparison
    #[inline(always)]
    fn calculate_write_value_bytes(
        dst_offset: usize,
        src_offset: usize,
        count: usize,
        pre_value: u64,
        src_values: &[u64],
    ) -> u64 {
        let mut result_bytes = pre_value.to_le_bytes();
        let src0_bytes = src_values[0].to_le_bytes();
        let src1_bytes = src_values[1].to_le_bytes();

        for i in 0..count {
            let src_idx = src_offset + i;
            result_bytes[dst_offset + i] =
                if src_idx < 8 { src0_bytes[src_idx] } else { src1_bytes[src_idx - 8] };
        }

        u64::from_le_bytes(result_bytes)
    }

    #[test]
    fn benchmark_calculate_write_value() {
        use std::time::Instant;

        let src0: u64 = 0x0102030405060708;
        let src1: u64 = 0x1112131415161718;
        let pre_value: u64 = 0xAABBCCDDEEFF0011;
        let src_values = [src0, src1];

        const ITERATIONS: usize = 1_000_000;

        // Warm up
        let mut sum_bitwise: u64 = 0;
        let mut sum_bytes: u64 = 0;

        // Benchmark bitwise implementation
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            for dst_offset in 0..8 {
                for count in 1..=(8 - dst_offset) {
                    for src_offset in 0..8 {
                        sum_bitwise = sum_bitwise.wrapping_add(DmaHelpers::calculate_write_value(
                            dst_offset,
                            src_offset,
                            count,
                            pre_value,
                            &src_values,
                        ));
                    }
                }
            }
        }
        let bitwise_duration = start.elapsed();

        // Benchmark byte-based implementation
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            for dst_offset in 0..8 {
                for count in 1..=(8 - dst_offset) {
                    for src_offset in 0..8 {
                        sum_bytes = sum_bytes.wrapping_add(calculate_write_value_bytes(
                            dst_offset,
                            src_offset,
                            count,
                            pre_value,
                            &src_values,
                        ));
                    }
                }
            }
        }
        let bytes_duration = start.elapsed();

        // Verify both produce same results
        assert_eq!(sum_bitwise, sum_bytes, "Results differ!");

        // 288 combinations per iteration (8 dst * varying count * 8 src)
        let total_ops = ITERATIONS * 288;

        println!("\n=== Benchmark Results ===");
        println!("Iterations: {} ({} total operations)", ITERATIONS, total_ops);
        println!("Bitwise implementation: {:?}", bitwise_duration);
        println!("Byte-based implementation: {:?}", bytes_duration);
        println!(
            "Bitwise ops/sec: {:.2}M",
            total_ops as f64 / bitwise_duration.as_secs_f64() / 1_000_000.0
        );
        println!(
            "Bytes ops/sec: {:.2}M",
            total_ops as f64 / bytes_duration.as_secs_f64() / 1_000_000.0
        );
        println!(
            "Speedup (bitwise vs bytes): {:.2}x",
            bytes_duration.as_secs_f64() / bitwise_duration.as_secs_f64()
        );
        println!("Checksum (to prevent optimization): {}", sum_bitwise);
    }

    #[test]
    fn asm_fast_encode_table() {
        let table = generate_fast_encode_table();
        for i in 0..256 {
            let dst_offset = i >> 5;
            let src_offset = (i >> 2) & 0x7;
            println!(
                "\t.quad 0x{:016x}, 0x{:016X}, 0x{:016X}, 0x{:016X} # {:4} - {:4} D{dst_offset} S{src_offset} C{}",
                table[i * 4],
                table[i * 4 + 1],
                table[i * 4 + 2],
                table[i * 4 + 3],
                i * 4,
                i * 4 + 3,
                (i * 4) & 0xF
            );
        }
        assert!(table.len() == 1024);
    }
    #[test]
    fn test_fast_encode_table() {
        for dst in 0..256 {
            for src in 0..256 {
                for count in 0..256 {
                    let encode = DmaInfo::encode_memcpy(dst, src, count);
                    let fast_encode = DmaInfo::fast_encode_memcpy(dst, src, count);
                    assert_eq!(
                        encode,
                        fast_encode,
                        "testing dst:0x{dst:08X} src:0x{src:08X} count:{count} E:0x{encode:016X} FE:0x{fast_encode:016X}"
                    );
                }
            }
        }
    }

    #[allow(dead_code)]
    fn benchmark_fast_encode_vs_encode_memcpy() {
        use std::time::Instant;

        const ITERATIONS: usize = 10_000_000;
        let mut checksum1 = DmaInfo::encode_memcpy(0, 0, 8);
        let mut checksum2 = DmaInfo::fast_encode_memcpy(0, 0, 8);

        // Benchmark encode_memcpy (original)
        let start = Instant::now();
        for i in 0..ITERATIONS {
            let dst = (i & 0xFF) as u64;
            let src = ((i >> 8) & 0xFF) as u64;
            let count = (i >> 16) & 0xFF;
            checksum1 ^= DmaInfo::encode_memcpy(dst, src, count);
        }
        let duration_encode = start.elapsed();

        // Benchmark fast_encode (table-based)
        let start = Instant::now();
        for i in 0..ITERATIONS {
            let dst = (i & 0xFF) as u64;
            let src = ((i >> 8) & 0xFF) as u64;
            let count = (i >> 16) & 0xFF;
            checksum2 ^= DmaInfo::fast_encode_memcpy(dst, src, count);
        }
        let duration_fast = start.elapsed();

        // Verify both produce same results
        assert_eq!(checksum1, checksum2, "Checksums must match!");

        // Print results
        println!("\n═══════════════════════════════════════════════════════");
        println!("  DMA Encoding Benchmark ({} iterations)", ITERATIONS);
        println!("═══════════════════════════════════════════════════════");
        println!(
            "  encode_memcpy:  {:?} ({:.2} ns/op)",
            duration_encode,
            duration_encode.as_nanos() as f64 / ITERATIONS as f64
        );
        println!(
            "  fast_encode:    {:?} ({:.2} ns/op)",
            duration_fast,
            duration_fast.as_nanos() as f64 / ITERATIONS as f64
        );
        println!("───────────────────────────────────────────────────────");
        let speedup = duration_encode.as_nanos() as f64 / duration_fast.as_nanos() as f64;
        println!("  Speedup:        {:.2}x faster", speedup);
        println!("═══════════════════════════════════════════════════════\n");

        // Assert that fast_encode is actually faster
        assert!(duration_fast < duration_encode, "fast_encode should be faster than encode_memcpy");
    }
}
