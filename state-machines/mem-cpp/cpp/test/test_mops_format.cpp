// The record-length and header decoding of the memory-ops stream on a synthetic chunk with one
// record of every kind of both forms, heavy and light. The same words and expectations are in
// tools/mops (Rust).
#include <stdio.h>
#include <stdint.h>
#include "../mem_types.hpp"

static const uint64_t WORDS[] = {
    0x8048d141a0000003ull, 0xc048d158a0000010ull, 0x0000000000000001ull, 0x84000014a0000006ull,
    0x8048d171a0000001ull, 0x000000000000005aull, 0x8048d14c80001000ull, 0x8048d14da0002000ull,
    0x0000000000000007ull, 0x8400000da0002008ull, 0x8000005a40000008ull, 0x0048d14000000000ull,
    0x8000064e40001000ull, 0x0048d14000000000ull, 0xc000007a40002000ull, 0xc000003ba0003000ull,
    0xc00000cfa0004000ull, 0x8000009fa0005000ull, 0x0048d14000000000ull, 0xaaf37bf7a0100000ull,
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
    0xaaf37817a0200000ull, 0x0000000000000000ull, 0x0000000000000042ull
};
struct Expect { uint32_t len; uint32_t addr; uint32_t mode; int count; const char* name; };
static const Expect EXPECT[] = {
    {1u, 0xa0000003u, 0x01u, -1, "read_1 unaligned"},
    {2u, 0xa0000010u, 0x18u, -1, "write_8 aligned, bit 63 value"},
    {1u, 0xa0000006u, 0x14u, -1, "write_4 no value"},
    {2u, 0xa0000001u, 0x31u, -1, "cwrite_1 with value"},
    {1u, 0x80001000u, 0x0cu, -1, "aligned read"},
    {2u, 0xa0002000u, 0x0du, -1, "aligned write with value"},
    {1u, 0xa0002008u, 0x0du, -1, "aligned write no value"},
    {2u, 0x40000008u, 0x0au, 5, "block read 5"},
    {2u, 0x40001000u, 0x0eu, 100, "aligned block read 100"},
    {1u, 0x40002000u, 0x0au, 7, "block read 7, no payload"},
    {1u, 0xa0003000u, 0x0bu, 3, "block write 3, no payload"},
    {1u, 0xa0004000u, 0x0fu, 12, "aligned block write 12, no payload"},
    {2u, 0xa0005000u, 0x0fu, 9, "aligned block write 9 with step payload"},
    {65u, 0xa0100000u, 0x0fu, 63, "value block 63"},
    {3u, 0xa0200000u, 0x0fu, 1, "value block 1"},
};

int main() {
    const uint32_t n = sizeof(WORDS) / sizeof(WORDS[0]);
    const uint32_t nrec = sizeof(EXPECT) / sizeof(EXPECT[0]);
    uint32_t k = 0, failures = 0;
    for (uint32_t r = 0; r < nrec; r++) {
        const Expect& e = EXPECT[r];
        if (k >= n) { printf("FAIL %s: stream ended\n", e.name); failures++; break; }
        const uint32_t len = mops_record_len(WORDS[k]);
        const MemCountersBusData d = mops_decode_record(&WORDS[k]);
        const uint32_t low = d.flags & 0x0F;
        const bool block = low >= 0x0A && low != 0x0C && low != 0x0D;
        const int count = block ? (int)(d.flags >> MOPS_BLOCK_COUNT_SBITS) : -1;
        const uint32_t mode = block ? low : (d.flags & 0x3F);
        if (len != e.len || d.addr != e.addr || mode != e.mode || count != e.count) {
            printf("FAIL %s: len %u/%u addr 0x%08x/0x%08x mode 0x%02x/0x%02x count %d/%d\n",
                   e.name, len, e.len, d.addr, e.addr, mode, e.mode, count, e.count);
            failures++;
        }
        k += len;
    }
    if (k != n) { printf("FAIL: %u words consumed of %u\n", k, n); failures++; }
    printf("%s: %u records, %u words%s\n", failures ? "FAILED" : "OK", nrec, n, failures ? "" : ", the decoders agree with the table");
    return failures ? 1 : 0;
}
