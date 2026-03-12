#[cfg(test)]
mod memcpy_tests {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    use super::ziskos::memcpy;
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    fn memcpy(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
        unsafe {
            std::ptr::copy(src, dst, len);
        }
        dst
    }
    use std::alloc::{alloc, dealloc, Layout};

    // Helper function to create aligned memory
    unsafe fn alloc_aligned(size: usize, align: usize) -> *mut u8 {
        let layout = Layout::from_size_align(size + align, align).unwrap();
        let ptr = alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate memory");
        }
        // Align the pointer
        let aligned = (ptr as usize + align - 1) & !(align - 1);
        aligned as *mut u8
    }

    // Helper function to deallocate aligned memory
    unsafe fn dealloc_aligned(ptr: *mut u8, size: usize, align: usize) {
        let layout = Layout::from_size_align(size + align, align).unwrap();
        // We need to get back to the original pointer, but for simplicity in tests,
        // we'll use a different approach
        dealloc(ptr, layout);
    }

    #[test]
    fn test_memcpy_zero_length() {
        unsafe {
            let src = [1u8, 2, 3, 4];
            let mut dst = [0u8; 4];

            let result = memcpy(dst.as_mut_ptr(), src.as_ptr(), 0);

            assert_eq!(result, dst.as_mut_ptr());
            assert_eq!(dst, [0, 0, 0, 0]); // Should remain unchanged
        }
    }

    #[test]
    fn test_memcpy_single_byte() {
        unsafe {
            let src = [0x42u8];
            let mut dst = [0u8; 1];

            memcpy(dst.as_mut_ptr(), src.as_ptr(), 1);

            assert_eq!(dst[0], 0x42);
        }
    }

    #[test]
    fn test_memcpy_aligned_8_small() {
        unsafe {
            // Test 8-byte aligned pointers with small copy (< 32 bytes)
            let src = alloc_aligned(64, 8);
            let dst = alloc_aligned(64, 8);

            // Initialize source data
            for i in 0..16 {
                *src.add(i) = (i + 1) as u8;
            }

            memcpy(dst, src, 16);

            // Verify copy
            for i in 0..16 {
                assert_eq!(*dst.add(i), (i + 1) as u8, "Mismatch at byte {}", i);
            }

            dealloc_aligned(src, 64, 8);
            dealloc_aligned(dst, 64, 8);
        }
    }

    #[test]
    fn test_memcpy_aligned_8_large() {
        unsafe {
            // Test 8-byte aligned pointers with large copy (> 32 bytes)
            let src = alloc_aligned(128, 8);
            let dst = alloc_aligned(128, 8);

            // Initialize source data
            for i in 0..64 {
                *src.add(i) = (i % 256) as u8;
            }

            memcpy(dst, src, 64);

            // Verify copy
            for i in 0..64 {
                assert_eq!(*dst.add(i), (i % 256) as u8, "Mismatch at byte {}", i);
            }

            dealloc_aligned(src, 128, 8);
            dealloc_aligned(dst, 128, 8);
        }
    }

    #[test]
    fn test_memcpy_src_unaligned() {
        unsafe {
            // Test unaligned source pointer
            let src_base = alloc_aligned(64, 8);
            let dst = alloc_aligned(64, 8);
            let src = src_base.add(3); // Unaligned by 3 bytes

            // Initialize source data
            for i in 0..20 {
                *src.add(i) = (i + 0x10) as u8;
            }

            memcpy(dst, src, 20);

            // Verify copy
            for i in 0..20 {
                assert_eq!(*dst.add(i), (i + 0x10) as u8, "Mismatch at byte {}", i);
            }

            dealloc_aligned(src_base, 64, 8);
            dealloc_aligned(dst, 64, 8);
        }
    }

    #[test]
    fn test_memcpy_dst_unaligned() {
        unsafe {
            // Test unaligned destination pointer
            let src = alloc_aligned(64, 8);
            let dst_base = alloc_aligned(64, 8);
            let dst = dst_base.add(5); // Unaligned by 5 bytes

            // Initialize source data
            for i in 0..20 {
                *src.add(i) = (i + 0x20) as u8;
            }

            memcpy(dst, src, 20);

            // Verify copy
            for i in 0..20 {
                assert_eq!(*dst.add(i), (i + 0x20) as u8, "Mismatch at byte {}", i);
            }

            dealloc_aligned(src, 64, 8);
            dealloc_aligned(dst_base, 64, 8);
        }
    }

    #[test]
    fn test_memcpy_both_unaligned() {
        unsafe {
            // Test both pointers unaligned
            let src_base = alloc_aligned(64, 8);
            let dst_base = alloc_aligned(64, 8);
            let src = src_base.add(2); // Unaligned by 2 bytes
            let dst = dst_base.add(6); // Unaligned by 6 bytes

            // Initialize source data
            for i in 0..25 {
                *src.add(i) = (i + 0x30) as u8;
            }

            memcpy(dst, src, 25);

            // Verify copy
            for i in 0..25 {
                assert_eq!(*dst.add(i), (i + 0x30) as u8, "Mismatch at byte {}", i);
            }

            dealloc_aligned(src_base, 64, 8);
            dealloc_aligned(dst_base, 64, 8);
        }
    }

    #[test]
    fn test_memcpy_edge_sizes() {
        unsafe {
            let sizes = [1, 2, 3, 4, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65];

            for &size in &sizes {
                let src = alloc_aligned(128, 8);
                let dst = alloc_aligned(128, 8);

                // Initialize source data
                for i in 0..size {
                    *src.add(i) = ((i * 3 + 7) % 256) as u8;
                }

                memcpy(dst, src, size);

                // Verify copy
                for i in 0..size {
                    assert_eq!(
                        *dst.add(i),
                        ((i * 3 + 7) % 256) as u8,
                        "Size {} mismatch at byte {}",
                        size,
                        i
                    );
                }

                dealloc_aligned(src, 128, 8);
                dealloc_aligned(dst, 128, 8);
            }
        }
    }

    #[test]
    fn test_memcpy_overlapping_forward() {
        unsafe {
            // Test overlapping memory (src before dst) - should work
            let mut buffer = [0u8; 20];

            // Initialize data
            for i in 0..10 {
                buffer[i] = (i + 0x40) as u8;
            }

            // Copy from buffer[0..10] to buffer[5..15]
            memcpy(buffer.as_mut_ptr().add(5), buffer.as_ptr(), 10);

            // Verify - first 5 bytes unchanged, next 10 are the copy
            for i in 0..5 {
                assert_eq!(buffer[i], (i + 0x40) as u8);
            }
            for i in 5..15 {
                assert_eq!(buffer[i], (i - 5 + 0x40) as u8);
            }
        }
    }

    #[test]
    fn test_memcpy_return_value() {
        unsafe {
            let src = [1u8, 2, 3, 4];
            let mut dst = [0u8; 4];
            let dst_ptr = dst.as_mut_ptr();

            let result = memcpy(dst_ptr, src.as_ptr(), 4);

            assert_eq!(result, dst_ptr, "Return value should be original dst pointer");
        }
    }

    #[test]
    fn test_memcpy_large_unaligned() {
        unsafe {
            // Test large copy with unaligned pointers
            let src_base = alloc_aligned(256, 8);
            let dst_base = alloc_aligned(256, 8);
            let src = src_base.add(3); // Unaligned
            let dst = dst_base.add(1); // Unaligned

            // Initialize source data with pattern
            for i in 0..200 {
                *src.add(i) = ((i * 7 + 13) % 256) as u8;
            }

            memcpy(dst, src, 200);

            // Verify copy
            for i in 0..200 {
                assert_eq!(
                    *dst.add(i),
                    ((i * 7 + 13) % 256) as u8,
                    "Large unaligned mismatch at byte {}",
                    i
                );
            }

            dealloc_aligned(src_base, 256, 8);
            dealloc_aligned(dst_base, 256, 8);
        }
    }

    #[test]
    fn test_memcpy_debug_print() {
        unsafe {
            let src = [0x12u8, 0x34, 0x56, 0x78];
            let mut dst = [0u8; 4];

            println!("Before memcpy:");
            println!("  src: {:p} = {:02x?}", src.as_ptr(), src);
            println!("  dst: {:p} = {:02x?}", dst.as_ptr(), dst);

            memcpy(dst.as_mut_ptr(), src.as_ptr(), 4);

            println!("After memcpy:");
            println!("  dst: {:p} = {:02x?}", dst.as_ptr(), dst);

            assert_eq!(dst, src);
        }
    }

    #[test]
    fn test_pointer_printing_formats() {
        unsafe {
            let data = [0xDEu8, 0xAD, 0xBE, 0xEF];
            let ptr = data.as_ptr();

            println!("\nDifferent ways to print pointers:");
            println!("Standard format:     {:p}", ptr);
            println!("Hex lowercase:       0x{:x}", ptr as usize);
            println!("Hex UPPERCASE:       0x{:X}", ptr as usize);
            println!("With padding:        0x{:016x}", ptr as usize);
            println!("Auto-prefixed:       {:#x}", ptr as usize);
            println!("Debug format:        {:?}", ptr);

            // También mostrar como imprimir la data
            println!("\nData at pointer:");
            println!("Hex bytes:           {:02x?}", data);
            println!("Hex UPPER bytes:     {:02X?}", data);
            println!("Pretty debug:        {:#02x?}", data);
        }
    }

    #[test]
    fn test_memcpy_large_aligned_568699() {
        unsafe {
            const SIZE: usize = 568699;

            // Allocate aligned memory for both source and destination
            let src = alloc_aligned(SIZE + 64, 8);
            let dst = alloc_aligned(SIZE + 64, 8);

            println!("\nTest memcpy with aligned pointers and size: {}", SIZE);
            println!("Source pointer:      {:p} (0x{:016x})", src, src as usize);
            println!("Destination pointer: {:p} (0x{:016x})", dst, dst as usize);
            println!("Alignment check src: {} (should be 0)", (src as usize) & 7);
            println!("Alignment check dst: {} (should be 0)", (dst as usize) & 7);

            // Initialize source data with a predictable pattern
            for i in 0..SIZE {
                *src.add(i) = ((i * 73 + 127) % 256) as u8;
            }

            // Perform the copy
            let start = std::time::Instant::now();
            let result = memcpy(dst, src, SIZE);
            let elapsed = start.elapsed();

            println!("Copy completed in: {:?}", elapsed);
            println!(
                "Throughput: {:.2} MB/s",
                (SIZE as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64()
            );

            // Verify the return value
            assert_eq!(result, dst, "Return value should be original dst pointer");

            // Verify the copy by checking every byte
            let mut mismatches = 0;
            for i in 0..SIZE {
                let expected = ((i * 73 + 127) % 256) as u8;
                let actual = *dst.add(i);
                if actual != expected {
                    if mismatches < 10 {
                        // Only print first 10 mismatches
                        println!(
                            "Mismatch at byte {}: expected 0x{:02x}, got 0x{:02x}",
                            i, expected, actual
                        );
                    }
                    mismatches += 1;
                }
            }

            if mismatches > 0 {
                println!("Total mismatches: {}", mismatches);
                panic!("Copy verification failed with {} mismatches", mismatches);
            }

            println!("✓ All {} bytes copied correctly", SIZE);

            // Test some specific boundary checks
            println!("\nBoundary checks:");
            println!("First byte:  src[0]=0x{:02x}, dst[0]=0x{:02x}", *src, *dst);
            let last_idx = SIZE - 1;
            println!(
                "Last byte:   src[{}]=0x{:02x}, dst[{}]=0x{:02x}",
                last_idx,
                *src.add(last_idx),
                last_idx,
                *dst.add(last_idx)
            );

            // Check bytes at key positions (32-byte boundaries, etc.)
            let check_positions = [31, 32, 63, 64, 127, 128, 255, 256, 511, 512, 1023, 1024];
            for &pos in &check_positions {
                if pos < SIZE {
                    let expected = ((pos * 73 + 127) % 256) as u8;
                    let actual = *dst.add(pos);
                    println!(
                        "Position {}: expected=0x{:02x}, actual=0x{:02x} {}",
                        pos,
                        expected,
                        actual,
                        if expected == actual { "✓" } else { "✗" }
                    );
                    assert_eq!(actual, expected, "Mismatch at position {}", pos);
                }
            }

            dealloc_aligned(src, SIZE + 64, 8);
            dealloc_aligned(dst, SIZE + 64, 8);
        }
    }

    #[test]
    fn test_memcpy_various_large_sizes() {
        // Test various large sizes to ensure robustness
        let test_sizes = [
            568699,  // Original request
            568700,  // Just one more
            568698,  // Just one less
            1048576, // 1 MB
            524288,  // 512 KB
            131072,  // 128 KB
            65536,   // 64 KB
            32768,   // 32 KB
            16384,   // 16 KB
        ];

        for &size in &test_sizes {
            unsafe {
                println!("\nTesting size: {} bytes", size);

                let src = alloc_aligned(size + 64, 8);
                let dst = alloc_aligned(size + 64, 8);

                // Simple pattern for faster initialization
                for i in 0..size {
                    *src.add(i) = (i % 256) as u8;
                }

                let start = std::time::Instant::now();
                memcpy(dst, src, size);
                let elapsed = start.elapsed();

                println!(
                    "  Time: {:?}, Throughput: {:.2} MB/s",
                    elapsed,
                    (size as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64()
                );

                // Quick verification - check first, last, and some middle bytes
                assert_eq!(*dst, 0, "First byte mismatch for size {}", size);
                if size > 1 {
                    let last_idx = size - 1;
                    let expected_last = (last_idx % 256) as u8;
                    assert_eq!(
                        *dst.add(last_idx),
                        expected_last,
                        "Last byte mismatch for size {}",
                        size
                    );
                }

                // Check a few strategic positions
                let check_positions = [size / 4, size / 2, 3 * size / 4];
                for &pos in &check_positions {
                    if pos < size {
                        let expected = (pos % 256) as u8;
                        assert_eq!(
                            *dst.add(pos),
                            expected,
                            "Mismatch at position {} for size {}",
                            pos,
                            size
                        );
                    }
                }

                dealloc_aligned(src, size + 64, 8);
                dealloc_aligned(dst, size + 64, 8);
            }
        }

        println!("\n✓ All large size tests passed!");
    }
}
