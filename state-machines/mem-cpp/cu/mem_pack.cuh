#ifndef MEM_PACK_CUH
#define MEM_PACK_CUH

// Packing of `Mem` trace rows on the device (ram_fill.cu).
//
// A packed row is `words_per_row` u64 words holding every column of every lane, in declaration
// order, at sequential bit offsets. Columns come in 11 groups; every group has one column per lane
// except `value`, which has two. The widths come from the trace the prover declares, so the kernel
// never hardcodes them.

#include <cstdint>

constexpr uint32_t MEMPACK_MAX_WORDS_PER_ROW = 16;
constexpr uint32_t MEMPACK_MAX_COLS = 64;
constexpr uint32_t MEMPACK_N_GROUPS = 11;
constexpr uint32_t MEMPACK_GROUP_VALUE = 5;

enum MemPackGroup : uint32_t {
    MP_ADDR = 0, MP_STEP = 1, MP_ADDR_CHANGES = 2, MP_STEP_DUAL = 3, MP_SEL_DUAL = 4, MP_VALUE = 5,
    MP_WR = 6, MP_PREVIOUS_STEP = 7, MP_L_INCREMENT = 8, MP_H_INCREMENT = 9, MP_READ_SAME_ADDR = 10,
};

struct MemPackLayout {
    uint32_t off[MEMPACK_MAX_COLS];
    uint32_t width[MEMPACK_MAX_COLS];
    uint32_t gbase[MEMPACK_N_GROUPS];
    uint32_t words_per_row;
    uint32_t lanes_x_row;
    uint32_t n_cols;
};

// Resolves the layout on the host. False when the widths do not fit the row or the column count
// does not match the lane count.
inline bool mempack_layout(MemPackLayout& l, const uint32_t* col_widths, uint32_t n_cols,
                           uint32_t words_per_row, uint32_t lanes_x_row) {
    if (words_per_row == 0 || words_per_row > MEMPACK_MAX_WORDS_PER_ROW || n_cols > MEMPACK_MAX_COLS ||
        lanes_x_row == 0 || n_cols != 12 * lanes_x_row)
        return false;
    uint32_t off = 0;
    for (uint32_t c = 0; c < n_cols; ++c) {
        l.off[c] = off;
        l.width[c] = col_widths[c];
        off += col_widths[c];
    }
    if (off > words_per_row * 64) return false;
    uint32_t cbase = 0;
    for (uint32_t g = 0; g < MEMPACK_N_GROUPS; ++g) {
        l.gbase[g] = cbase;
        cbase += (g == MEMPACK_GROUP_VALUE ? 2 : 1) * lanes_x_row;
    }
    l.words_per_row = words_per_row;
    l.lanes_x_row = lanes_x_row;
    l.n_cols = n_cols;
    return true;
}

__device__ __forceinline__ void mempack_put_bits(uint64_t* w, uint32_t off, uint32_t width, uint64_t v) {
    const uint32_t ws = off >> 6, bs = off & 63;
    const uint64_t mask = width >= 64 ? ~0ull : ((1ull << width) - 1);
    v &= mask;
    w[ws] |= v << bs;
    if (bs + width > 64) w[ws + 1] |= v >> (64 - bs);
}

__device__ __forceinline__ void mempack_put(const MemPackLayout& l, uint64_t* w, uint32_t group,
                                            uint32_t lane, uint32_t idx, uint64_t v) {
    const uint32_t c = l.gbase[group] + (group == MEMPACK_GROUP_VALUE ? lane * 2 + idx : lane);
    mempack_put_bits(w, l.off[c], l.width[c], v);
}

// One lane's worth of data, as the pack kernels see it.
struct MemPackLane {
    uint32_t addr;
    uint64_t step;
    uint64_t value;
    bool wr;
    bool dual;
    uint64_t step_dual;
};

// Writes the lane of an access. `addr_changes` and `inc` follow `fill_mem_range` in mem_sm.rs.
__device__ __forceinline__ void mempack_lane(const MemPackLayout& l, uint64_t* w, uint32_t lane,
                                             const MemPackLane& me, bool addr_changes, uint64_t inc) {
    mempack_put(l, w, MP_ADDR, lane, 0, me.addr);
    mempack_put(l, w, MP_STEP, lane, 0, me.step);
    mempack_put(l, w, MP_ADDR_CHANGES, lane, 0, addr_changes ? 1 : 0);
    mempack_put(l, w, MP_STEP_DUAL, lane, 0, me.step_dual);
    mempack_put(l, w, MP_SEL_DUAL, lane, 0, me.dual ? 1 : 0);
    mempack_put(l, w, MP_VALUE, lane, 0, (uint32_t)me.value);
    mempack_put(l, w, MP_VALUE, lane, 1, (uint32_t)(me.value >> 32));
    mempack_put(l, w, MP_WR, lane, 0, me.wr ? 1 : 0);
    mempack_put(l, w, MP_L_INCREMENT, lane, 0, inc & ((1ull << 22) - 1));
    mempack_put(l, w, MP_H_INCREMENT, lane, 0, (inc >> 22) & 0xFFFF);
}

// A padding lane (`MemPadding` in mem_sm.rs): a read of the last word of the region, `addr`, at
// the last step. `changes` is set on the first padding lane when the last access sat elsewhere:
// the lane then changes address and carries the address distance in `inc`.
__device__ __forceinline__ void mempack_padding(const MemPackLayout& l, uint64_t* w, uint32_t lane,
                                                uint32_t addr, uint64_t step, uint64_t value, bool changes,
                                                uint64_t inc) {
    mempack_put(l, w, MP_ADDR, lane, 0, addr);
    mempack_put(l, w, MP_STEP, lane, 0, step);
    mempack_put(l, w, MP_ADDR_CHANGES, lane, 0, changes ? 1 : 0);
    mempack_put(l, w, MP_VALUE, lane, 0, (uint32_t)value);
    mempack_put(l, w, MP_VALUE, lane, 1, (uint32_t)(value >> 32));
    mempack_put(l, w, MP_L_INCREMENT, lane, 0, inc & ((1ull << 22) - 1));
    mempack_put(l, w, MP_H_INCREMENT, lane, 0, (inc >> 22) & 0xFFFF);
    mempack_put(l, w, MP_READ_SAME_ADDR, lane, 0, changes ? 0 : 1);
}

// The increment of an access lane given the previous lane (or the previous segment when `v0`).
// `prev_addr` is the previous lane's address, or the continuation's address; `prev_last_step` is
// the previous lane's step, or step_dual when dual.
__device__ __forceinline__ uint64_t mempack_increment(bool addr_changes, const MemPackLane& me,
                                                      uint32_t prev_addr, uint64_t prev_last_step) {
    if (addr_changes) return (uint64_t)me.addr - (uint64_t)prev_addr - 1;
    return me.step - prev_last_step - (me.wr ? 1 : 0);
}

#endif  // MEM_PACK_CUH
