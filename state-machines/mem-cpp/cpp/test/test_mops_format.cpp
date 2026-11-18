// The record-length and header decoding of the memory-ops stream on a synthetic chunk with one
// record of every kind of both forms, heavy and light. The words are frozen numbers, not built
// from the format constants, so the table is a second copy of the format spec: a decoder that
// drifts from it fails here. The step column is read with the mops_format.hpp field helpers the
// GPU planner shares; the last record pins the value-block step's top bit (header bit 62), which
// a mode-up flags extract (29 bits, bits 33-61) silently drops.
#include <stdio.h>
#include <stdint.h>
#include "../mem_types.hpp"

static const uint64_t WORDS[] = {
    0x8091a282a0000003ull, 0xc091a2b0a0000010ull, 0x0000000000000001ull, 0x88000028a0000006ull,
    0x8091a2e2a0000001ull, 0x000000000000005aull, 0x8091a29880001000ull, 0x8091a29aa0002000ull,
    0x0000000000000007ull, 0x8800001aa0002008ull, 0x800000b440000008ull, 0x0091a28000000000ull,
    0x80000c9c40001000ull, 0x0091a28000000000ull, 0xc00000f440002000ull, 0xc0000076a0003000ull,
    0xc000019ea0004000ull, 0x8000013ea0005000ull, 0x0091a28000000000ull, 0xd5e6f7eea0100000ull,
    0x0000000000000020ull, 0x0000000000000000ull, 0x0101010101010101ull, 0x0202020202020202ull,
    0x0303030303030303ull, 0x0404040404040404ull, 0x0505050505050505ull, 0x0606060606060606ull,
    0x0707070707070707ull, 0x0808080808080808ull, 0x0909090909090909ull, 0x0a0a0a0a0a0a0a0aull,
    0x0b0b0b0b0b0b0b0bull, 0x0c0c0c0c0c0c0c0cull, 0x0d0d0d0d0d0d0d0dull, 0x0e0e0e0e0e0e0e0eull,
    0x0f0f0f0f0f0f0f0full, 0x1010101010101010ull, 0x1111111111111111ull, 0x1212121212121212ull,
    0x1313131313131313ull, 0x1414141414141414ull, 0x1515151515151515ull, 0x1616161616161616ull,
    0x1717171717171717ull, 0x1818181818181818ull, 0x1919191919191919ull, 0x1a1a1a1a1a1a1a1aull,
    0x1b1b1b1b1b1b1b1bull, 0x1c1c1c1c1c1c1c1cull, 0x1d1d1d1d1d1d1d1dull, 0x1e1e1e1e1e1e1e1eull,
    0x1f1f1f1f1f1f1f1full, 0x2020202020202020ull, 0x2121212121212121ull, 0x2222222222222222ull,
    0x2323232323232323ull, 0x2424242424242424ull, 0x2525252525252525ull, 0x2626262626262626ull,
    0x2727272727272727ull, 0x2828282828282828ull, 0x2929292929292929ull, 0x2a2a2a2a2a2a2a2aull,
    0x2b2b2b2b2b2b2b2bull, 0x2c2c2c2c2c2c2c2cull, 0x2d2d2d2d2d2d2d2dull, 0x2e2e2e2e2e2e2e2eull,
    0x2f2f2f2f2f2f2f2full, 0x3030303030303030ull, 0x3131313131313131ull, 0x3232323232323232ull,
    0x3333333333333333ull, 0x3434343434343434ull, 0x3535353535353535ull, 0x3636363636363636ull,
    0x3737373737373737ull, 0x3838383838383838ull, 0x3939393939393939ull, 0x3a3a3a3a3a3a3a3aull,
    0x3b3b3b3b3b3b3b3bull, 0x3c3c3c3c3c3c3c3cull, 0x3d3d3d3d3d3d3d3dull, 0x3e3e3e3e3e3e3e3eull,
    0xd5e6f02ea0200000ull, 0x0000000000000000ull, 0x0000000000000042ull, 0xc091a2d010000000ull,
    0x0000000000000042ull, 0x8800005010000000ull, 0x8091a2913ffffff8ull, 0xfffff82ec0000000ull,
    0x0000000000000000ull, 0x4242424242424242ull
};
struct Expect { uint32_t len; uint64_t addr; uint32_t mode; int count; int64_t step; const char* name; };
static const Expect EXPECT[] = {
    {1u, 0x0a0000003ull, 0x01u, -1, 0x12345, "read_1 unaligned"},
    {2u, 0x0a0000010ull, 0x18u, -1, 0x12345, "write_8 aligned, bit 63 value"},
    {1u, 0x0a0000006ull, 0x14u, -1, 0x00000, "write_4 no value"},
    {2u, 0x0a0000001ull, 0x31u, -1, 0x12345, "cwrite_1 with value"},
    {1u, 0x080001000ull, 0x0cu, -1, 0x12345, "aligned read"},
    {2u, 0x0a0002000ull, 0x0du, -1, 0x12345, "aligned write with value"},
    {1u, 0x0a0002008ull, 0x0du, -1, 0x00000, "aligned write no value"},
    {2u, 0x040000008ull, 0x0au, 5, 0x12345, "block read 5"},
    {2u, 0x040001000ull, 0x0eu, 100, 0x12345, "aligned block read 100"},
    {1u, 0x040002000ull, 0x0au, 7, -1, "block read 7, no payload"},
    {1u, 0x0a0003000ull, 0x0bu, 3, -1, "block write 3, no payload"},
    {1u, 0x0a0004000ull, 0x0fu, 12, -1, "aligned block write 12, no payload"},
    {2u, 0x0a0005000ull, 0x0fu, 9, 0x12345, "aligned block write 9 with step payload"},
    {65u, 0x0a0100000ull, 0x0fu, 63, 0xabcde, "value block 63"},
    {3u, 0x0a0200000ull, 0x0fu, 1, 0xabcde, "value block 1"},
    {2u, 0x010000000ull, 0x28u, -1, 0x12345, "free-input read with value"},
    {1u, 0x010000000ull, 0x28u, -1, 0x00000, "free-input read, light"},
    {1u, 0x13ffffff8ull, 0x08u, -1, 0x12345, "read_8 at the last word of RAM"},
    {3u, 0x0c0000000ull, 0x0fu, 1, 0xfffff, "value block, step field all ones (header bit 62)"},
};

int main() {
    const uint32_t n = sizeof(WORDS) / sizeof(WORDS[0]);
    const uint32_t nrec = sizeof(EXPECT) / sizeof(EXPECT[0]);
    uint32_t k = 0, failures = 0;
    for (uint32_t r = 0; r < nrec; r++) {
        const Expect& e = EXPECT[r];
        if (k >= n) { printf("FAIL %s: stream ended\n", e.name); failures++; break; }
        const uint64_t hdr = WORDS[k];
        const uint32_t len = mops_record_len(hdr);
        const MemCountersBusData d = mops_decode_record(&WORDS[k]);
        const uint32_t low = d.flags & 0x0F;
        const bool block = low >= 0x0A && low != 0x0C && low != 0x0D;
        const int count = block ? (int)(d.flags >> MOPS_BLOCK_COUNT_SBITS) : -1;
        const uint32_t mode = block ? low : (d.flags & 0x3F);
        // The step, as the GPU planner reads it (the CPU decode above does not carry it): value
        // blocks keep it in the header above the flags bits, other blocks in the payload word.
        const uint32_t raw_low = (uint32_t)(hdr >> MOPS_MODE_SHIFT) & 0x0F;
        int64_t step;
        if (raw_low == MOPS_BLOCK_VALUES) step = mops_values_step_field(hdr);
        else if (raw_low >= 0x0A && raw_low != 0x0C && raw_low != 0x0D)
            step = len == 2 ? (int64_t)mops_hdr_step_field(WORDS[k + 1]) : -1;
        else step = (int64_t)mops_hdr_step_field(hdr);
        if (len != e.len || d.addr != e.addr || mode != e.mode || count != e.count || step != e.step) {
            printf("FAIL %s: len %u/%u addr 0x%09lx/0x%09lx mode 0x%02x/0x%02x count %d/%d step 0x%lx/0x%lx\n",
                   e.name, len, e.len, (unsigned long)d.addr, (unsigned long)e.addr, mode, e.mode,
                   count, e.count, (long)step, (long)e.step);
            failures++;
        }
        k += len;
    }
    if (k != n) { printf("FAIL: %u words consumed of %u\n", k, n); failures++; }
    printf("%s: %u records, %u words%s\n", failures ? "FAILED" : "OK", nrec, n, failures ? "" : ", the decoders agree with the table");
    return failures ? 1 : 0;
}
