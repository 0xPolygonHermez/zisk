#include <cstdint>
#include <cstring>
#include <iostream>
#include <vector>
#include <iomanip>
#include <cassert>

#define NO_OVERLAPPING 0xFFFFFFFF

// External assembly function declarations
extern "C" {
    uint64_t dma_memcpy_mtrace(uint64_t dst, uint64_t src, uint64_t count, uint64_t* trace_ptr);
    uint64_t dma_memcpy_mops(uint64_t dst, uint64_t src, uint64_t count, uint64_t* mops_ptr);
    void dma_memcpy_fast(uint64_t dst, uint64_t src, uint64_t count);
    uint64_t fast_dma_encode(uint64_t dst, uint64_t src, uint64_t count);
}
const char *mops_labels[16] = {"NOP", "CWR1", "RD1", "WR1", "RD2", "WR2", "RD4", "WR4", "RD8", "WR8",
                             "ARD", "AWR", "BR", "BW", "ABR", "ABW"};

// MOPS constants from dma_constants.inc
const uint64_t EXTRA_PARAMETER_ADDR = 0xA000'0F00;
const uint64_t MOPS_ALIGNED_READ = 0x0000'000C'0000'0000ULL;
const uint64_t MOPS_ALIGNED_BLOCK_READ = 0x0000'000E'0000'0000ULL;
const uint64_t MOPS_ALIGNED_BLOCK_WRITE = 0x0000'000F'0000'0000ULL;
const uint64_t MOPS_BLOCK_WORDS_SBITS = 36;

class Memory {
protected:
    uint8_t *original_bytes;
public:
    uint8_t *bytes;
    size_t size;
    Memory (size_t size): size(size) {
        bytes = (uint8_t *) aligned_alloc(8, size);
        if (bytes == NULL) {
            printf("Failed to allocate bytes\n");
            exit(1);
        }
        if ((((uint64_t) bytes) & 0x07) != 0) {
            printf("Invalid allocation %p\n", bytes);
            exit(1);
        }
        original_bytes = (uint8_t *) aligned_alloc(8, size);
        if (original_bytes == NULL) {
            printf("Failed to allocate original_bytes\n");
            free(bytes);
            exit(1);
        }
    }
    Memory(const Memory&) = delete;
    Memory& operator=(const Memory&) = delete;
    ~Memory() {
        if (original_bytes) free(original_bytes);
        if (bytes) free(bytes);
        original_bytes = NULL;
        bytes = NULL;
    }

    const uint8_t *get_original_bytes(uint8_t *reference) {
        return original_bytes + (reference - bytes);
    }
    void fill_pattern(uint8_t start = 0) {
        for (size_t i = 0; i < size; ++i) {
            bytes[i] = start + i;
        }
        memcpy(original_bytes, bytes, size);
    }

    bool verify_pattern(uint8_t start = 0, const char *title = "") {
        for (size_t i = 0; i < size; ++i) {
            uint8_t expected = start + i;
            if (bytes[i] != expected) {
                printf("FAIL PATTERN VERIFICATION of %s: Expected: 0x%02X vs data[%ld]=0x%02X\n",
                    title, expected, i, bytes[i]);
                return false;
            }
        }
        return true;
    }

    bool verify_pattern_except(uint8_t start, size_t addr, size_t count, const char *title = "") {
        size_t from = addr - (size_t)bytes;
        size_t to = from + count - 1;
        for (size_t i = 0; i < size; ++i) {
            if (i >= from && i <= to) continue;
            uint8_t expected = start + i;
            if (bytes[i] != expected) {
                printf("FAIL PATTERN VERIFICATION of %s: Expected: 0x%02X vs data[%ld]=0x%02X\n",
                    title, expected, i, bytes[i]);
                return false;
            }
        }
        return true;
    }
};

// Helper class to manage aligned test buffers
class AlignedBuffer {
public:
    std::vector<uint8_t> data;
    
    AlignedBuffer(size_t size) : data(size, 0) {}
    
    uint64_t* aligned_ptr() {
        return reinterpret_cast<uint64_t*>(data.data());
    }
    
    uint8_t* byte_ptr() {
        return data.data();
    }
    
    void fill_pattern(uint8_t start = 0) {
        for (size_t i = 0; i < data.size(); ++i) {
            data[i] = static_cast<uint8_t>(start + i);
        }
    }

    bool verify_pattern(uint8_t start = 0, const char *title = "") {
        for (size_t i = 0; i < data.size(); ++i) {
            uint8_t expected = static_cast<uint8_t>(start + i);
            if (data[i] != expected) {
                printf("FAIL PATTERN VERIFICATION of %s: Expected: 0x%02X vs data[%ld]=0x%02X\n",
                    title, expected, i, data[i]);
                return false;
            }
        }
        return true;
    }

    bool verify_pattern_except(uint8_t start, size_t from, size_t count, const char *title = "") {
        size_t to = from + count - 1;
        for (size_t i = 0; i < data.size(); ++i) {
            if (i >= from && i <= to) continue;
            uint8_t expected = static_cast<uint8_t>(start + i);
            if (data[i] != expected) {
                printf("FAIL PATTERN VERIFICATION of %s: Expected: 0x%02X vs data[%ld]=0x%02X\n",
                    title, expected, i, data[i]);
                return false;
            }
        }
        return true;
    }

    bool verify_fill(uint8_t value, const char *title = "") {
        for (size_t i = 0; i < data.size(); ++i) {
            if (data[i] != value) {
                printf("FAIL PATTERN VERIFICATION of %s: Expected: 0x%02X vs data[%ld]:0x%02X\n",
                    title, value, i, data[i]);
                return false;
            }
        }
        return true;
    }

    bool verify_fill_except(uint8_t value, size_t from, size_t count, const char *title = "") {
        size_t to = from + count - 1;
        for (size_t i = 0; i < data.size(); ++i) {
            if (i >= from && i <= to) continue;
            if (data[i] != value) {
                printf("FAIL PATTERN VERIFICATION of %s: Expected: 0x%02X vs data[%ld]:0x%02X\n", 
                    title, value, i, data[i]);
                return false;
            }
        }
        return true;
    }
    
    void fill_value(uint8_t value) {
        std::fill(data.begin(), data.end(), value);
    }
};

#define TRACE_EQ(EXPECTED, CALCULATED, MSG) \
    if (EXPECTED != CALCULATED) { \
        fprintf(stderr, "❌ FAIL: Trace comparation on %s (E: 0x%016lX vs 0x%016lX) \n", MSG, EXPECTED, CALCULATED); \
        exit(1); \
    } 

// Reference implementation for encode_memcpy from Rust  
uint64_t encode_memcpy_reference(uint64_t dst, uint64_t src, uint64_t count) {
    uint64_t dst_offset = dst & 0x07;
    uint64_t src_offset = src & 0x07;
    
    uint64_t pre_count, loop_count, post_count;
    
    if (dst_offset > 0) {
        uint64_t _pre_count = 8 - dst_offset;
        if (_pre_count >= count) {
            pre_count = count;
            loop_count = 0;
            post_count = 0;
        } else {
            uint64_t pending = count - _pre_count;
            pre_count = _pre_count;
            loop_count = pending >> 3;
            post_count = pending & 0x07;
        }
    } else {
        pre_count = 0;
        loop_count = count >> 3;
        post_count = count & 0x07;
    }
    
    uint64_t pre_writes = (pre_count > 0) + (post_count > 0);
    uint64_t src_offset_pos = (src_offset + pre_count) & 0x07;
    uint64_t double_src_post = (src_offset_pos + post_count) > 8;
    uint64_t double_src_pre = (src_offset + pre_count) > 8;
    uint64_t extra_src_reads = 
        (count == 0) ? 0 : ((((src + count - 1) >> 3) - (src >> 3) + 1) - loop_count);
    uint64_t src64_inc_by_pre = (pre_count > 0 && (src_offset + pre_count) >= 8);
    uint64_t unaligned_dst_src = (src_offset != dst_offset);

    return pre_count     
        | (post_count << 3)
        | (pre_writes << 6)
        | (dst_offset << 8)
        | (src_offset << 11)
        | (double_src_pre << 14)
        | (double_src_post << 15)
        | (extra_src_reads << 16)
        | (src64_inc_by_pre << 18)
        | (unaligned_dst_src << 19)
        | (pre_count << 29)
        | (loop_count << 32);
}

// Extract fields from encoded value
struct EncodedInfo {
    uint64_t loop_count;
    uint64_t pre_writes;
    uint64_t dst_offset;
    uint64_t src_offset;
    uint64_t pre_count;
    uint64_t post_count;
    bool double_src_pre;
    bool double_src_post;
    uint64_t extra_src_reads;
    uint64_t src64_inc_by_pre;
    uint64_t unaligned_dst_src;
    
    EncodedInfo(uint64_t encoded) {
        loop_count = encoded >> 32;
        pre_count = encoded & 0x07;
        post_count = (encoded >> 3) & 0x07;
        pre_writes = (encoded >> 6) & 0x03;
        dst_offset = (encoded >> 8) & 0x07;
        src_offset = (encoded >> 11) & 0x07;
        double_src_pre = (encoded >> 14) & 0x01;
        double_src_post = (encoded >> 15) & 0x01;
        extra_src_reads = (encoded >> 16) & 0x03;
        src64_inc_by_pre = (encoded >> 18) & 0x01;
        unaligned_dst_src = (encoded >> 19) & 0x01;
    }
    
    void print() const {
        std::cout << "  loop_count: " << loop_count << "\n"
                  << "  pre_writes: " << pre_writes << "\n"
                  << "  dst_offset: " << dst_offset << "\n"
                  << "  src_offset: " << src_offset << "\n"
                  << "  pre_count: " << pre_count << "\n"
                  << "  post_count: " << post_count << "\n"
                  << "  double_src_pre: " << double_src_pre << "\n"
                  << "  double_src_post: " << double_src_post << "\n"
                  << "  extra_src_reads: " << extra_src_reads << "\n"
                  << "  src64_inc_by_pre: " << src64_inc_by_pre << "\n"
                  << "  unaligned_dst_src: " << unaligned_dst_src << "\n";
    }
};

void print_hex_dump(const char* label, const uint8_t* data, size_t size) {
    std::cout << label << " (" << size << " bytes):\n";
    for (size_t i = 0; i < size; i += 16) {
        std::cout << "  " << std::hex << std::setw(4) << std::setfill('0') << i << ": ";
        for (size_t j = 0; j < 16 && i + j < size; ++j) {
            std::cout << std::hex << std::setw(2) << std::setfill('0') 
                      << static_cast<int>(data[i + j]) << " ";
        }
        std::cout << "\n";
    }
    std::cout << std::dec;
}

bool test_memcpy_mtrace(Memory &mem, uint64_t dst_offset, uint64_t src_offset, size_t count,  
                        const char* description, int64_t overlapping = NO_OVERLAPPING) {


    bool is_overlapping = overlapping != NO_OVERLAPPING;
    if (is_overlapping) {
        printf("\n\x1b[1;36m##### test_memcpy_mtrace(%ld, %ld, %ld,\"%s\", overlapping:%ld) #####\x1b[0m\n", 
            dst_offset, src_offset, count, description, overlapping);
    } else {
        printf("\n\x1b[1;36m##### test_memcpy_mtrace(%ld, %ld, %ld,\"%s\") #####\x1b[0m\n", 
            dst_offset, src_offset, count, description);
    }
    mem.fill_pattern(0x10);

    uint8_t *src;
    uint8_t *dst;

    if (is_overlapping) {
        src = mem.bytes + 1024;
        dst = src + overlapping;
    } else {
        src = mem.bytes + 1024;
        dst = src + ((count + 7) & ~0x07) + 1024;
    }
    AlignedBuffer trace_buf(4096);
    
    // Clear trace buffer
    trace_buf.fill_value(0);

    if (!mem.verify_pattern(0x10) ||  !trace_buf.verify_fill(0, "trace_buff")) {
        return false;
    }
    
    uint64_t src_addr = ((uint64_t) src) + src_offset;
    uint64_t dst_addr = ((uint64_t) dst) + dst_offset;
    uint64_t* trace_ptr = trace_buf.aligned_ptr();

    printf("TEST dst:0x%08lX src:0x%08lX count:%ld  trace:%p\n", dst_addr, src_addr, count, trace_ptr);
    
    // Calculate reference encoding BEFORE assembly call to avoid register corruption
    uint64_t encoded_ref = encode_memcpy_reference(dst_addr, src_addr, count);    
    
    // Call assembly function and capture return value
    uint64_t qwords_written = dma_memcpy_mtrace(dst_addr, src_addr, count, trace_ptr);
    
    // Get encoded value from trace
    uint64_t encoded_asm = trace_ptr[0];
    
    if (encoded_asm != encoded_ref) {
        printf("Encoded (ASM): 0x%016lX\n", encoded_asm);
        printf("Encoded (REF): 0x%016lX\n", encoded_ref);
        
        std::cerr << "❌ FAIL: Encoded value mismatch!\n";
        EncodedInfo info_asm(encoded_asm);
        EncodedInfo info_ref(encoded_ref);
        std::cout << "ASM info:\n";
        info_asm.print();
        std::cout << "REF info:\n";
        info_ref.print();
        return false;
    }
    
    // Verify the actual memcpy was performed
    const uint8_t* src_bytes;
    if (is_overlapping) {
        src_bytes = mem.get_original_bytes((uint8_t *)src_addr);
    } else {
        src_bytes = (uint8_t *)src_addr;
    }
    const uint8_t* dst_bytes = reinterpret_cast<const uint8_t*>(dst_addr);
    
    bool copy_ok = true;
    for (size_t i = 0; i < count; ++i) {
        if (dst_bytes[i] != src_bytes[i]) {
            std::cerr << "❌ FAIL: Memory copy mismatch at byte " << i << "\n";
            std::cerr << "  Expected: 0x" << std::hex << static_cast<int>(src_bytes[i])
                      << ", Got: 0x" << static_cast<int>(dst_bytes[i]) << std::dec << "\n";
            copy_ok = false;
            break;
        }
    }
    
    if (!copy_ok) {
        print_hex_dump("Source", src_bytes, std::min(count, size_t(64)));
        print_hex_dump("Destination", dst_bytes, std::min(count, size_t(64)));
        return false;
    }
    
    std::cout << "✅ PASS: Encoding and copy correct\n";
    
    // Print trace buffer summary
    EncodedInfo info(encoded_asm);
    size_t trace_idx = 1;
    std::cout << "Trace buffer:\n";
    std::cout << "  [0] Encoded: 0x" << std::hex << trace_ptr[0] << std::dec << "\n";
    
    uint64_t *dst_original = (uint64_t*) mem.get_original_bytes((uint8_t *) (dst_addr & ~0x07));
    if (info.pre_count > 0) {
        TRACE_EQ(dst_original[0], trace_ptr[trace_idx], "PRE pre-write value not match");
        trace_idx++;
    }
    
    if (info.post_count > 0) {
        size_t last_dst_index = (dst_offset + count - 1) >> 3;
        TRACE_EQ(dst_original[last_dst_index], trace_ptr[trace_idx], "POST pre-write value not match");
        trace_idx++;
    }
    
    size_t expected_src_qwords = info.loop_count + info.extra_src_reads;
    // Verify that the function returned the correct number of qwords written
    size_t expected_total_qwords = trace_idx + expected_src_qwords;
    
    if (qwords_written != expected_total_qwords) {
        std::cerr << "❌ FAIL: Incorrect number of qwords returned!\n";
        std::cerr << "  Expected: " << expected_total_qwords << " qwords\n";
        std::cerr << "  Got: " << qwords_written << " qwords\n";
        return false;
    }
    
    uint64_t *src_original = (uint64_t*) mem.get_original_bytes((uint8_t *) (src_addr & ~0x07));
    for (size_t index = 0; index < expected_src_qwords; ++index) {
        TRACE_EQ(src_original[index], trace_ptr[trace_idx], "SRC values not match");
        trace_idx++;        
    }
    if (!mem.verify_pattern_except(0x10, dst_addr, count, "mem (out)") || 
        !trace_buf.verify_fill_except(0, 0, qwords_written * 8, "trace_buff (out)")) {
        return false;
    }
    std::cout << "✅ Returned correct qword count: " << qwords_written << " qwords\n";
    
    return true;
}

bool test_memcpy_mops(Memory &mem, uint64_t dst_offset, uint64_t src_offset, size_t count,  
                      const char* description, int64_t overlapping = NO_OVERLAPPING) {

    
    bool is_overlapping = overlapping != NO_OVERLAPPING;
    if (is_overlapping) {
        printf("\n\x1b[1;35m##### test_memcpy_mops(%ld, %ld, %ld,\"%s\", overlapping:%ld) #####\x1b[0m\n", 
            dst_offset, src_offset, count, description, overlapping);
    } else {
        printf("\n\x1b[1;35m##### test_memcpy_mops(%ld, %ld, %ld,\"%s\") #####\x1b[0m\n", 
            dst_offset, src_offset, count, description);
    }
    mem.fill_pattern(0x10);

    uint8_t *src;
    uint8_t *dst;

    if (is_overlapping) {
        src = mem.bytes + 1024;
        dst = src + overlapping;
    } else {
        src = mem.bytes + 1024;
        dst = src + ((count + 7) & ~0x07) + 1024;
    }
    AlignedBuffer mops_buf(4096);
    
    // Clear mops buffer
    mops_buf.fill_value(0);

    if (!mem.verify_pattern(0x10) ||  !mops_buf.verify_fill(0, "mops_buff")) {
        return false;
    }
    
    uint64_t src_addr = ((uint64_t) src) + src_offset;
    uint64_t dst_addr = ((uint64_t) dst) + dst_offset;
    uint64_t* mops_ptr = mops_buf.aligned_ptr();

    printf("TEST dst:0x%08lX src:0x%08lX count:%ld  mops:%p\n", dst_addr, src_addr, count, mops_ptr);
    
    // Calculate reference encoding to know expected structure
    uint64_t encoded_ref = encode_memcpy_reference(dst_addr, src_addr, count);
    EncodedInfo info(encoded_ref);
    printf("INFO pre:%ld%s post:%ld%s loop:%ld sibp:%ld\n", info.pre_count, info.double_src_pre ? "+D":"", 
            info.post_count,  info.double_src_post ? "+D":"", info.loop_count, info.src64_inc_by_pre);
    
    // Call assembly function and capture return value
    uint64_t mops_entries = dma_memcpy_mops(dst_addr, src_addr, count, mops_ptr);
    
    // Verify the actual memcpy was performed
    const uint8_t* src_bytes;
    if (is_overlapping) {
        src_bytes = mem.get_original_bytes((uint8_t *)src_addr);
    } else {
        src_bytes = (uint8_t *)src_addr;
    }
    const uint8_t* dst_bytes = reinterpret_cast<const uint8_t*>(dst_addr);
    
    bool copy_ok = true;
    for (size_t i = 0; i < count; ++i) {
        if (dst_bytes[i] != src_bytes[i]) {
            std::cerr << "❌ FAIL: Memory copy mismatch at byte " << i << "\n";
            std::cerr << "  Expected: 0x" << std::hex << static_cast<int>(src_bytes[i])
                      << ", Got: 0x" << static_cast<int>(dst_bytes[i]) << std::dec << "\n";
            copy_ok = false;
            break;
        }
    }
    
    if (!copy_ok) {
        print_hex_dump("Source", src_bytes, std::min(count, size_t(64)));
        print_hex_dump("Destination", dst_bytes, std::min(count, size_t(64)));
        return false;
    }
    
    std::vector<std::pair<uint64_t, std::string>> expected; 

    expected.emplace_back(MOPS_ALIGNED_READ + EXTRA_PARAMETER_ADDR, "PARAM count");
    
    if (info.pre_count > 0) {
        expected.emplace_back(MOPS_ALIGNED_READ + (dst_addr & ~0x07ULL), "PRE preread dst");
        expected.emplace_back((info.double_src_pre ? (MOPS_ALIGNED_BLOCK_READ + (2ULL << MOPS_BLOCK_WORDS_SBITS)): MOPS_ALIGNED_READ) +
                              (src_addr & ~0x07ULL), "PRE src read");
    }

    if (info.post_count > 0) {
        expected.emplace_back(MOPS_ALIGNED_READ + ((dst_addr + count - 1) & ~0x07ULL), "POST preread dst");
        expected.emplace_back((info.double_src_post ? (MOPS_ALIGNED_BLOCK_READ + (2ULL << MOPS_BLOCK_WORDS_SBITS)): MOPS_ALIGNED_READ) + 
                              ((src_addr + info.pre_count + info.loop_count * 8) & ~0x07ULL), "POST src read");
    }
    
    if (info.loop_count > 0) {
        expected.emplace_back(MOPS_ALIGNED_BLOCK_READ + ((info.loop_count + (info.unaligned_dst_src)) << MOPS_BLOCK_WORDS_SBITS) 
                              + ((src_addr + info.pre_count) & ~0x07ULL), "LOOP read src");
    }
    
    if (count > 0) {
        expected.emplace_back(MOPS_ALIGNED_BLOCK_WRITE + ((info.loop_count + info.pre_writes) << MOPS_BLOCK_WORDS_SBITS) 
                              + (dst_addr & ~0x07ULL), "write dst");
    }

    size_t max_entries = std::max(mops_entries, expected.size());
    bool errors = false;
    for (size_t i = 0; i < max_entries; ++i) {
        printf("MOPS[%2ld] = ", i);
        if (i < mops_entries) {
            printf("%3ld %3s 0x%08lX  # ", mops_ptr[i] >> MOPS_BLOCK_WORDS_SBITS, 
                mops_labels[(mops_ptr[i] >> 32) & 0x0F], mops_ptr[i] & 0xFFFF'FFFF);
        } else {
            printf("--- --- ----------  #");        
        }
        if (i < expected.size()) {
            printf("%3ld %3s 0x%08lX %s", expected[i].first >> MOPS_BLOCK_WORDS_SBITS, 
                mops_labels[(expected[i].first >> 32) & 0x0F], expected[i].first & 0xFFFF'FFFF, expected[i].second.c_str());
        }
        if (i >= mops_entries || i >= expected.size()) {
            printf(" \x1B[31;1mFAIL\x1B[0m\n");
            errors = true;
        } else if (mops_ptr[i] != expected[i].first) {
            printf(" \x1B[31;1mNOT MATCH\x1B[0m\n");
            errors = true;
        } else {
            printf("\n");
        }
    }
    
    // Verify total mops entries count
    if (mops_entries != expected.size()) {
        printf("FAIL: Incorrect number of mops entries (E:%ld vs %ld)", expected.size(), mops_entries);
        return false;
    }

    if (errors) {
        return false;
    }
    std::cout << "✅ PASS: MOPS entries and copy correct (" << mops_entries << " entries)\n";
    
    if (!mem.verify_pattern_except(0x10, dst_addr, count, "mem (out)") || 
        !mops_buf.verify_fill_except(0, 0, mops_entries * 8, "mops_buff (out)")) {
        return false;
    }
    
    return true;
}

bool test_overlapping_copy(const char* description, int64_t offset) {
    std::cout << "\n=== Test: " << description << " ===\n";
    std::cout << "Offset: " << offset << " bytes\n";
    
    AlignedBuffer buf(2048);
    AlignedBuffer trace_buf(2048);
    
    // Fill with pattern
    buf.fill_pattern(0x20);
    trace_buf.fill_value(0);
    
    size_t count = 32;
    uint64_t src_addr = reinterpret_cast<uint64_t>(buf.byte_ptr() + 64);
    uint64_t dst_addr = src_addr + offset;
    
    // Validate bounds
    if (dst_addr < reinterpret_cast<uint64_t>(buf.byte_ptr()) || 
        dst_addr + count > reinterpret_cast<uint64_t>(buf.byte_ptr() + buf.data.size())) {
        std::cerr << "❌ FAIL: dst_addr out of bounds\n";
        return false;
    }
    uint64_t* trace_ptr = trace_buf.aligned_ptr();
    
    // Save original data
    std::vector<uint8_t> original_src(count);
    std::memcpy(original_src.data(), reinterpret_cast<void*>(src_addr), count);
    
    // Call function
    dma_memcpy_mtrace(dst_addr, src_addr, count, trace_ptr);
    
    // Verify copy
    const uint8_t* dst_bytes = reinterpret_cast<const uint8_t*>(dst_addr);
    bool ok = true;
    for (size_t i = 0; i < count; ++i) {
        if (dst_bytes[i] != original_src[i]) {
            std::cerr << "❌ FAIL: Overlapping copy mismatch at byte " << i << "\n";
            ok = false;
            break;
        }
    }
    
    if (ok) {
        std::cout << "✅ PASS: Overlapping copy correct\n";
    }
    
    return ok;
}

bool test_fast_memcpy(uint64_t dst_offset, uint64_t src_offset, size_t count,
                      const char* description) {
    std::cout << "\n=== Test Fast: " << description << " ===\n";
    std::cout << "dst_offset=" << dst_offset << ", src_offset=" << src_offset
              << ", count=" << count << "\n";
    
    // Allocate buffers
    AlignedBuffer src_buf(2048);
    AlignedBuffer dst_buf(2048);
    
    // Fill source with pattern
    src_buf.fill_pattern(0x10);
    // Fill destination with different pattern
    dst_buf.fill_pattern(0xA0);
    
    // Calculate actual addresses with offsets
    uint64_t src_addr = reinterpret_cast<uint64_t>(src_buf.byte_ptr() + 64) + src_offset;
    uint64_t dst_addr = reinterpret_cast<uint64_t>(dst_buf.byte_ptr() + 64) + dst_offset;
    
    // Call assembly function (no trace)
    dma_memcpy_fast(dst_addr, src_addr, count);
    
    // Verify the memcpy was performed
    const uint8_t* src_bytes = reinterpret_cast<const uint8_t*>(src_addr);
    const uint8_t* dst_bytes = reinterpret_cast<const uint8_t*>(dst_addr);
    
    bool copy_ok = true;
    for (size_t i = 0; i < count; ++i) {
        if (dst_bytes[i] != src_bytes[i]) {
            std::cout << "❌ FAIL: Mismatch at byte " << i << "\n";
            std::cout << "  Expected: 0x" << std::hex << (int)src_bytes[i]
                      << " Got: 0x" << (int)dst_bytes[i] << std::dec << "\n";
            copy_ok = false;
            break;
        }
    }
    
    if (!copy_ok) {
        return false;
    }
    
    std::cout << "✅ PASS: Fast copy correct\n";
    return true;
}

bool test_fast_overlapping_heap(const char* description, int64_t offset) {
    std::cout << "\n=== Test Fast Overlap (HEAP): " << description << " ===\n";
    std::cout << "Offset: " << offset << " bytes\n";
    
    size_t count = 32;
    size_t buffer_size = 4096;
    
    // Allocate with aligned_alloc to get proper alignment and avoid vector overhead
    uint8_t* buf = (uint8_t*)aligned_alloc(8, buffer_size);
    if (!buf) {
        std::cerr << "❌ FAIL: allocation failed\n";
        return false;
    }
    
    // Fill with pattern
    for (size_t i = 0; i < buffer_size; ++i) {
        buf[i] = 0x20 + (i & 0xFF);
    }
    
    uint64_t src_addr = reinterpret_cast<uint64_t>(buf + 1024);
    uint64_t dst_addr = src_addr + offset;
    
    // Validate bounds
    if (dst_addr < reinterpret_cast<uint64_t>(buf) || 
        dst_addr + count > reinterpret_cast<uint64_t>(buf + buffer_size)) {
        std::cerr << "❌ FAIL: dst_addr out of bounds\n";
        free(buf);
        return false;
    }
    
    // Set canaries
    size_t min_addr = std::min(src_addr, dst_addr) - reinterpret_cast<uint64_t>(buf);
    size_t max_addr = std::max(src_addr + count, dst_addr + count) - reinterpret_cast<uint64_t>(buf);
    
    const uint8_t CANARY = 0xCA;
    for (size_t i = min_addr - 8; i < min_addr; ++i) {
        buf[i] = CANARY;
    }
    for (size_t i = max_addr; i < max_addr + 8; ++i) {
        buf[i] = CANARY;
    }
    
    // Save original data
    uint8_t original_src[32];
    const uint8_t* src_bytes = reinterpret_cast<const uint8_t*>(src_addr);
    for (size_t i = 0; i < count; ++i) {
        original_src[i] = src_bytes[i];
    }
    
    // Call function
    printf("dma_memcpy_fast(0x%016lx,0x%016lx,%ld)\n", dst_addr, src_addr, count);
    dma_memcpy_fast(dst_addr, src_addr, count);
    printf("dma_memcpy_fast-END\n");
    
    // Check canaries
    bool canaries_ok = true;
    for (size_t i = min_addr - 8; i < min_addr; ++i) {
        if (buf[i] != CANARY) {
            std::cerr << "❌ FAIL: Canary corrupted BEFORE at offset " << i << "\n";
            canaries_ok = false;
        }
    }
    for (size_t i = max_addr; i < max_addr + 8; ++i) {
        if (buf[i] != CANARY) {
            std::cerr << "❌ FAIL: Canary corrupted AFTER at offset " << i << "\n";
            canaries_ok = false;
        }
    }
    
    // Verify result
    const uint8_t* dst_bytes = reinterpret_cast<const uint8_t*>(dst_addr);
    bool ok = canaries_ok;
    for (size_t i = 0; i < count; ++i) {
        if (dst_bytes[i] != original_src[i]) {
            std::cout << "❌ FAIL: Mismatch at byte " << i << "\n";
            ok = false;
            break;
        }
    }
    
    free(buf);
    
    if (ok) {
        std::cout << "✅ PASS: Fast overlapping copy correct (heap)\n";
    }
    
    return ok;
}

bool test_fast_overlapping(const char* description, int64_t offset) {
    std::cout << "\n=== Test Fast Overlap: " << description << " ===\n";
    std::cout << "Offset: " << offset << " bytes\n";
    
    size_t count = 32;
    
    // Use statically allocated aligned buffer with canaries
    static uint8_t static_buf[4096] __attribute__((aligned(8)));
    
    // Fill with pattern
    for (size_t i = 0; i < sizeof(static_buf); ++i) {
        static_buf[i] = 0x20 + (i & 0xFF);
    }
    
    // Calculate addresses with safety margins
    size_t guard_size = 64;  // 64 bytes before and after
    uint64_t src_addr = reinterpret_cast<uint64_t>(static_buf + 1024);
    uint64_t dst_addr = src_addr + offset;
    
    // Validate bounds
    if (dst_addr < reinterpret_cast<uint64_t>(static_buf + guard_size) || 
        dst_addr + count > reinterpret_cast<uint64_t>(static_buf + sizeof(static_buf) - guard_size)) {
        std::cerr << "❌ FAIL: dst_addr out of bounds\n";
        return false;
    }
    
    // Set canary values around the operation zone
    size_t min_addr = std::min(src_addr, dst_addr) - reinterpret_cast<uint64_t>(static_buf);
    size_t max_addr = std::max(src_addr + count, dst_addr + count) - reinterpret_cast<uint64_t>(static_buf);
    
    // Fill canaries before and after
    const uint8_t CANARY = 0xCA;
    for (size_t i = min_addr - 8; i < min_addr; ++i) {
        static_buf[i] = CANARY;
    }
    for (size_t i = max_addr; i < max_addr + 8; ++i) {
        static_buf[i] = CANARY;
    }
    
    // Save original data using simple array
    uint8_t original_src[32];
    const uint8_t* src_bytes = reinterpret_cast<const uint8_t*>(src_addr);
    for (size_t i = 0; i < count; ++i) {
        original_src[i] = src_bytes[i];
    }
    // std::memcpy(original_src.data(), reinterpret_cast<const void*>(src_addr), count);
    
    // Call fast function
    printf("dma_memcpy_fast(0x%016lx,0x%016lx,%ld)\n", dst_addr, src_addr, count);
    dma_memcpy_fast(dst_addr, src_addr, count);
    printf("dma_memcpy_fast-END\n");
    
    // Check canaries for buffer overflow
    bool canaries_ok = true;
    for (size_t i = min_addr - 8; i < min_addr; ++i) {
        if (static_buf[i] != CANARY) {
            std::cerr << "❌ FAIL: Canary corrupted BEFORE region at offset " << i 
                      << " (expected 0x" << std::hex << (int)CANARY 
                      << ", got 0x" << (int)static_buf[i] << std::dec << ")\n";
            canaries_ok = false;
        }
    }
    for (size_t i = max_addr; i < max_addr + 8; ++i) {
        if (static_buf[i] != CANARY) {
            std::cerr << "❌ FAIL: Canary corrupted AFTER region at offset " << i 
                      << " (expected 0x" << std::hex << (int)CANARY 
                      << ", got 0x" << (int)static_buf[i] << std::dec << ")\n";
            canaries_ok = false;
        }
    }
    
    if (!canaries_ok) {
        std::cerr << "❌ BUFFER OVERFLOW DETECTED!\n";
        std::cerr << "  src_addr: 0x" << std::hex << src_addr << std::dec << "\n";
        std::cerr << "  dst_addr: 0x" << std::hex << dst_addr << std::dec << "\n";
        std::cerr << "  count: " << count << "\n";
        std::cerr << "  offset: " << offset << "\n";
        return false;
    }
    
    // Verify result
    const uint8_t* dst_bytes = reinterpret_cast<const uint8_t*>(dst_addr);
    bool ok = true;
    for (size_t i = 0; i < count; ++i) {
        if (dst_bytes[i] != original_src[i]) {
            std::cout << "❌ FAIL: Mismatch at byte " << i << "\n";
            ok = false;
            break;
        }
    }
    
    if (!ok) {
        return false;
    }
    
    std::cout << "✅ PASS: Fast overlapping copy correct\n";
    return true;
}

int main() {
    Memory mem (8192);
    std::cout << "Testing DMA memory operations assembly implementation\n";
    std::cout << "=====================================================\n";
    
    int passed = 0;
    int total = 0;
    
    // Test mtrace (memory trace with full data)
    std::cout << "\n\x1b[1;33m=== MTRACE Tests (Full Memory Trace) ===\x1b[0m\n";
    auto run_mtrace_test = [&](uint64_t dst_off, uint64_t src_off, size_t count, const char* desc) {
        total++;
        if (test_memcpy_mtrace(mem, dst_off, src_off, count, desc)) {
            passed++;
        }
    };
    
    run_mtrace_test(0, 0, 0, "Zero count");
    run_mtrace_test(0, 0, 1, "Single byte, aligned");
    run_mtrace_test(0, 0, 8, "One qword, aligned");
    run_mtrace_test(0, 0, 16, "Two qwords, aligned");
    run_mtrace_test(1, 0, 7, "dst_offset=1, count=7");
    run_mtrace_test(7, 0, 1, "dst_offset=7, count=1");
    run_mtrace_test(7, 0, 2, "dst_offset=7, count=2");
    run_mtrace_test(3, 5, 10, "dst_offset=3, src_offset=5, count=10");
    run_mtrace_test(0, 0, 100, "Large aligned copy");
    run_mtrace_test(3, 5, 100, "Large unaligned copy");
    
    // Test mops (memory operations - addresses only)
    std::cout << "\n\x1b[1;33m=== MOPS Tests (Memory Operations) ===\x1b[0m\n";
    auto run_mops_test = [&](uint64_t dst_off, uint64_t src_off, size_t count, const char* desc) {
        total++;
        if (test_memcpy_mops(mem, dst_off, src_off, count, desc)) {
            passed++;
        } else {
            exit(1);
        }
    };
    
    run_mops_test(0, 0, 0, "Zero count");
    run_mops_test(0, 0, 1, "Single byte, aligned");
    run_mops_test(0, 0, 8, "One qword, aligned");
    run_mops_test(0, 0, 16, "Two qwords, aligned");
    run_mops_test(1, 0, 7, "dst_offset=1, count=7");
    run_mops_test(7, 0, 1, "dst_offset=7, count=1");
    run_mops_test(7, 0, 2, "dst_offset=7, count=2");
    run_mops_test(3, 5, 10, "dst_offset=3, src_offset=5, count=10");
    run_mops_test(0, 0, 100, "Large aligned copy");
    run_mops_test(3, 5, 100, "Large unaligned copy");
   
    // Comprehensive test
    std::cout << "\n=== Comprehensive Test ===\n";
    for (uint64_t dst_off = 0; dst_off < 8; ++dst_off) {
        for (uint64_t src_off = 0; src_off < 8; ++src_off) {
            for (size_t count = 0; count < 128; ++count) {
                total++;
                if (test_memcpy_mtrace(mem, dst_off, src_off, count, "Comprehensive")) {
                    passed++;
                }
                total++;
                if (test_memcpy_mtrace(mem, dst_off, src_off, count, "Comprehensive overlapping 0", 0)) {
                    passed++;
                }
                // total++;
                // if (count > 4) {
                //     if (test_encode_memcpy(mem, dst_off, src_off, count, "Comprehensive overlapping -4", -4)) {
                //         passed++;
                //     }
                //     total++;
                //     if (test_encode_memcpy(mem, dst_off, src_off, count, "Comprehensive overlapping 4", 4)) {
                //         passed++;
                //     }
                //     total++;
                //     if (test_encode_memcpy(mem, dst_off, src_off, count, "Comprehensive overlapping 1 byte", count-1)) {
                //         passed++;
                //     }
                // }
            }
        }
    }
    // Overlapping tests
    total++;
    if (test_overlapping_copy("Forward overlap (dst > src)", 8)) passed++;
    
    total++;
    if (test_overlapping_copy("Backward overlap (dst < src)", -8)) passed++;
    
    total++;
    if (test_overlapping_copy("No overlap (large gap)", 100)) passed++;
    
    // Fast memcpy tests
    std::cout << "\n=== Fast Memcpy Tests ===\n";
    auto run_fast_test = [&](uint64_t dst_off, uint64_t src_off, size_t count, const char* desc) {
        total++;
        if (test_fast_memcpy(dst_off, src_off, count, desc)) {
            passed++;
        }
    };
    
    run_fast_test(0, 0, 0, "Zero count");
    run_fast_test(0, 0, 1, "Single byte, aligned");
    run_fast_test(0, 0, 8, "One qword, aligned");
    run_fast_test(0, 0, 16, "Two qwords, aligned");
    run_fast_test(1, 0, 7, "dst_offset=1, count=7");
    run_fast_test(7, 0, 1, "dst_offset=7, count=1");
    run_fast_test(3, 5, 10, "dst_offset=3, src_offset=5, count=10");
    run_fast_test(0, 0, 100, "Large aligned copy");
    run_fast_test(3, 5, 100, "Large unaligned copy");
    run_fast_test(1, 2, 1000, "Very large copy");
    
    // Fast overlapping tests
    total++;
    if (test_fast_overlapping("Forward overlap (dst > src)", 8)) passed++;
    
    // Test with heap allocation
    total++;
    if (test_fast_overlapping_heap("Forward overlap (dst > src) HEAP", 8)) passed++;
    

    total++;
    if (test_fast_overlapping("Backward overlap (dst < src)", -8)) passed++;
    
    total++;
    if (test_fast_overlapping("No overlap (large gap)", 100)) passed++;

    // Summary
    std::cout << "\n=== Test Summary ===\n";
    std::cout << "Passed: " << passed << "/" << total << " tests\n";
    std::cout << "Success rate: " << (100.0 * passed / total) << "%\n";
    
    if (passed == total) {
        std::cout << "\n✅ All tests passed!\n";
        return 0;
    } else {
        std::cout << "\n❌ Some tests failed!\n";
        return 1;
    }
}
