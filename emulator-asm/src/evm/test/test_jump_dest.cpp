// Differential tests for the three jump_dest assembly variants against
// the reference walk, checking the bitmap, the minimal trace and the mops.

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#define EXTRA_PARAMETER_ADDR 0xA0400F00

#define MOPS_ALIGNED_READ 0x0C
#define MOPS_ALIGNED_BLOCK_READ 0x0E
#define MOPS_ALIGNED_BLOCK_WRITE 0x0F
#define MOPS_BLOCK_COUNT_SBITS 4

extern "C" {
uint64_t test_asm_jump_dest_fast(uint64_t *dst, const uint8_t *src, size_t count,
                                        uint64_t *trace);
uint64_t test_asm_jump_dest_mtrace(uint64_t *dst, const uint8_t *src, size_t count,
                                          uint64_t *trace);
uint64_t test_asm_jump_dest_mops(uint64_t *dst, const uint8_t *src, size_t count,
                                        uint64_t *trace);
uint64_t test_asm_jump_dest_mtrace_checked(uint64_t *dst, const uint8_t *src, size_t count,
                                                  uint64_t *trace);
uint64_t test_asm_jump_dest_mops_checked(uint64_t *dst, const uint8_t *src, size_t count,
                                                uint64_t *trace);
}

static const uint64_t GUARD = 0xAAAAAAAAAAAAAAAAull;

static size_t src_words(size_t count) { return (count + 7) / 8; }
static size_t bitmap_words(size_t count) { return (count + 63) / 64; }
static size_t max_read_runs(size_t count) { return (src_words(count) + 1) / 2; }

// ---------------------------------------------------------------------------
// Reference model: the word-granular walk the AIR proves, mirroring
// precompiles/evm/src/jump_dest.rs.
// ---------------------------------------------------------------------------

struct Expected {
    std::vector<uint64_t> bitmap;
    std::vector<size_t> reads;             // loaded source words, in order
    std::vector<std::pair<size_t, size_t>> runs;  // (first word, length)
};

static Expected reference(const uint8_t *code, size_t count) {
    Expected out;
    out.bitmap.assign(bitmap_words(count), 0);

    unsigned state = 0;
    for (size_t w = 0; w < src_words(count); ++w) {
        if (state >= 8) {
            state -= 8;                    // whole word is PUSH data: not loaded
            continue;
        }
        out.reads.push_back(w);
        if (!out.runs.empty() && out.runs.back().first + out.runs.back().second == w) {
            ++out.runs.back().second;
        } else {
            out.runs.push_back({w, 1});
        }

        const size_t valid = std::min<size_t>(8, count - w * 8);
        size_t i = state;
        uint8_t byte = 0;
        while (i < valid) {
            const uint8_t op = code[w * 8 + i];
            if (op == 0x5b) {
                byte |= (uint8_t)(1u << i);
                ++i;
            } else if ((op & 0xe0) == 0x60) {
                i += (size_t)op - 0x5e;
            } else {
                ++i;
            }
        }
        state = (unsigned)(i >= 8 ? i - 8 : 0);
        if (byte) out.bitmap[w / 8] |= (uint64_t)byte << (8 * (w % 8));
    }
    return out;
}

// Independent byte-at-a-time walk, the shape of the C++ guest builder. Used to
// confirm the word-granular reference itself is right.
static std::vector<uint64_t> byte_walk(const uint8_t *code, size_t count) {
    std::vector<uint64_t> bitmap(bitmap_words(count), 0);
    size_t p = 0;
    while (p < count) {
        const uint8_t op = code[p];
        if (op == 0x5b) {
            bitmap[p / 64] |= 1ull << (p % 64);
            ++p;
        } else if ((op & 0xe0) == 0x60) {
            p += (size_t)op - 0x5e;
        } else {
            ++p;
        }
    }
    return bitmap;
}

// ---------------------------------------------------------------------------

static uint64_t encode_aligned_read(uint64_t addr) {
    return ((uint64_t)MOPS_ALIGNED_READ << 32) | addr;
}
static uint64_t encode_block(uint64_t op, uint64_t addr, uint64_t words) {
    return (op << 32) | (words << (MOPS_BLOCK_COUNT_SBITS + 32)) | addr;
}

static std::string decode(uint64_t value) {
    const uint32_t flags = (uint32_t)(value >> 32);
    const uint32_t addr = (uint32_t)value;
    char buffer[128];
    snprintf(buffer, sizeof(buffer), "op:0x%02X words:%u addr:0x%08X", flags & 0x0F,
             flags >> MOPS_BLOCK_COUNT_SBITS, addr);
    return buffer;
}

// ---------------------------------------------------------------------------

class Test {
    static const size_t MAX_COUNT = 4096;

    uint8_t *src;
    uint64_t *dst;
    uint64_t *trace;
    std::vector<uint8_t> src_copy;
    size_t dst_words;
    size_t failures = 0;
    size_t cases = 0;
    std::string pattern_name;
    size_t count = 0;

    bool fail(const char *what) {
        printf("\n\x1B[1;31mFAIL\x1B[0m [%s count=%zu] %s\n", pattern_name.c_str(), count, what);
        ++failures;
        return false;
    }

    void reset_buffers() {
        for (size_t i = 0; i < dst_words; ++i) dst[i] = GUARD;
        memset(trace, 0, (MAX_COUNT + 64) * sizeof(uint64_t));
    }

    // Every bitmap word must be written and nothing past them touched.
    bool check_bitmap(const Expected &expected) {
        for (size_t i = 0; i < expected.bitmap.size(); ++i) {
            if (dst[i] != expected.bitmap[i]) {
                char buffer[160];
                snprintf(buffer, sizeof(buffer),
                         "bitmap[%zu] expected 0x%016lX found 0x%016lX", i,
                         (unsigned long)expected.bitmap[i], (unsigned long)dst[i]);
                return fail(buffer);
            }
        }
        for (size_t i = expected.bitmap.size(); i < dst_words; ++i) {
            if (dst[i] != GUARD) {
                char buffer[160];
                snprintf(buffer, sizeof(buffer), "wrote past the bitmap at word %zu", i);
                return fail(buffer);
            }
        }
        if (memcmp(src, src_copy.data(), src_copy.size()) != 0) {
            return fail("the bytecode was modified");
        }
        return true;
    }

public:
    Test() {
        posix_memalign((void **)&src, 64, MAX_COUNT + 64);
        dst_words = MAX_COUNT / 64 + 4;
        posix_memalign((void **)&dst, 64, dst_words * sizeof(uint64_t));
        posix_memalign((void **)&trace, 64, (MAX_COUNT + 64) * sizeof(uint64_t));
    }
    ~Test() {
        free(src);
        free(dst);
        free(trace);
    }

    void run_case(const std::string &name, const std::vector<uint8_t> &code);
    void run();
    size_t failed() const { return failures; }
    size_t total() const { return cases; }
};

void Test::run_case(const std::string &name, const std::vector<uint8_t> &code) {
    pattern_name = name;
    count = code.size();
    ++cases;

    memset(src, 0, MAX_COUNT + 64);
    memcpy(src, code.data(), count);
    src_copy.assign(src, src + MAX_COUNT + 64);

    const Expected expected = reference(src, count);

    // The word-granular reference must agree with the plain byte walk.
    if (expected.bitmap != byte_walk(src, count)) {
        fail("reference model disagrees with the byte walk");
        return;
    }
    // And the read grouping must respect the bound the buffer check relies on.
    if (expected.runs.size() > max_read_runs(count)) {
        fail("reference exceeded the worst-case run bound");
        return;
    }

    // ----- fast -----
    reset_buffers();
    uint64_t result = test_asm_jump_dest_fast(dst, src, count, trace);
    if (result != (uint64_t)dst) {
        fail("fast: wrong return value");
        return;
    }
    if (!check_bitmap(expected)) return;

    // ----- mtrace -----
    reset_buffers();
    result = test_asm_jump_dest_mtrace(dst, src, count, trace);
    if (result != (uint64_t)dst) {
        fail("mtrace: wrong return value");
        return;
    }
    if (!check_bitmap(expected)) return;

    // The payload is the whole contiguous source range, not just the loaded
    // words, so its length follows from count alone.
    const uint64_t *mt = trace + 1;
    const size_t mt_len = trace[0];
    const size_t payload = count ? src_words(count) : 0;
    if (mt_len != 1 + payload) {
        char buffer[160];
        snprintf(buffer, sizeof(buffer), "mtrace: expected %zu qwords, found %zu", 1 + payload,
                 mt_len);
        fail(buffer);
        return;
    }
    if (mt[0] != (uint64_t)count) {
        char buffer[160];
        snprintf(buffer, sizeof(buffer), "mtrace: header expected %zu found 0x%016lX", count,
                 (unsigned long)mt[0]);
        fail(buffer);
        return;
    }
    for (size_t w = 0; w < payload; ++w) {
        uint64_t word;
        memcpy(&word, src + w * 8, sizeof(word));
        if (mt[1 + w] != word) {
            char buffer[192];
            snprintf(buffer, sizeof(buffer),
                     "mtrace: src word %zu expected 0x%016lX found 0x%016lX", w,
                     (unsigned long)word, (unsigned long)mt[1 + w]);
            fail(buffer);
            return;
        }
    }

    // ----- mops -----
    reset_buffers();
    result = test_asm_jump_dest_mops(dst, src, count, trace);
    if (result != (uint64_t)dst) {
        fail("mops: wrong return value");
        return;
    }
    if (!check_bitmap(expected)) return;

    std::vector<uint64_t> want;
    want.push_back(encode_aligned_read(EXTRA_PARAMETER_ADDR));
    if (count > 0) {
        for (const auto &run : expected.runs) {
            want.push_back(encode_block(MOPS_ALIGNED_BLOCK_READ,
                                        (uint64_t)src + run.first * 8, run.second));
        }
        want.push_back(
            encode_block(MOPS_ALIGNED_BLOCK_WRITE, (uint64_t)dst, expected.bitmap.size()));
    }

    const uint64_t *mops = trace + 1;
    if (trace[0] != want.size()) {
        char buffer[160];
        snprintf(buffer, sizeof(buffer), "mops: expected %zu entries, found %zu", want.size(),
                 (size_t)trace[0]);
        fail(buffer);
        return;
    }
    for (size_t i = 0; i < want.size(); ++i) {
        if (mops[i] != want[i]) {
            char buffer[256];
            snprintf(buffer, sizeof(buffer), "mops[%zu]: expected %s found %s", i,
                     decode(want[i]).c_str(), decode(mops[i]).c_str());
            fail(buffer);
            return;
        }
    }
    // The bound the buffer reservation is built on.
    if (trace[0] > 2 + max_read_runs(count)) {
        fail("mops: entry count exceeded the reserved worst case");
        return;
    }

    // ----- the _with_count_check entries must behave identically -----
    reset_buffers();
    test_asm_jump_dest_mtrace_checked(dst, src, count, trace);
    if (!check_bitmap(expected)) return;
    if (trace[0] != 1 + payload) {
        fail("mtrace_checked: different trace length");
        return;
    }
    reset_buffers();
    test_asm_jump_dest_mops_checked(dst, src, count, trace);
    if (!check_bitmap(expected)) return;
    if (trace[0] != want.size()) {
        fail("mops_checked: different mops count");
        return;
    }
}

void Test::run() {
    // Deterministic xorshift, so a failure is always reproducible.
    uint64_t seed = 0x20260806;
    auto next = [&seed]() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        return seed;
    };

    const std::vector<uint8_t> pool = [] {
        std::vector<uint8_t> p(30, 0x5b);
        for (int op = 0x60; op <= 0x7f; ++op) {
            p.insert(p.end(), 3, (uint8_t)op);
        }
        for (uint8_t b : {0x00, 0x5a, 0x5f, 0x80, 0xff}) {
            p.insert(p.end(), 4, b);
        }
        return p;
    }();

    for (size_t count = 0; count <= 600; ++count) {
        printf("\rcount %4zu", count);
        fflush(stdout);

        run_case("all-jumpdest", std::vector<uint8_t>(count, 0x5b));
        run_case("all-zero", std::vector<uint8_t>(count, 0x00));
        run_case("all-push32", std::vector<uint8_t>(count, 0x7f));

        // Worst case for the mops entry count: PUSH15 on byte 7 of every other
        // word, so loads alternate on / off and no two runs can merge.
        std::vector<uint8_t> alternating(count, 0x00);
        for (size_t p = 7; p < count; p += 16) alternating[p] = 0x6e;
        run_case("alternating", alternating);

        // PUSH32 straddling the very end of the code.
        std::vector<uint8_t> straddle(count, 0x5b);
        if (count >= 3) straddle[count - 3] = 0x7f;
        run_case("straddle-end", straddle);

        std::vector<uint8_t> random(count);
        for (size_t i = 0; i < count; ++i) random[i] = pool[next() % pool.size()];
        run_case("random", random);

        if (failures) return;
    }

    // A few large cases, including a full 24KB-class contract.
    for (size_t count : {1024ul, 2049ul, 4000ul, 4096ul}) {
        std::vector<uint8_t> random(count);
        for (size_t i = 0; i < count; ++i) random[i] = pool[next() % pool.size()];
        run_case("large-random", random);
        if (failures) return;
    }
}

int main() {
    printf("\x1B[1;34mTEST EVM JUMP_DEST "
           "==========================================\x1B[0m\n");
    Test test;
    test.run();
    if (test.failed()) {
        printf("\n%zu case(s) \x1B[1;31mFAILED\x1B[0m\n", test.failed());
        return 1;
    }
    printf("\rAll %zu cases are [\x1B[1;32mOK\x1B[0m]%20s\n", test.total(), "");
    return 0;
}
