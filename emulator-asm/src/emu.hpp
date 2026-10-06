#ifndef EMU_ASM_HPP
#define EMU_ASM_HPP

#include <stdint.h>
#include <sys/time.h>

uint64_t TimeDiff(const struct timeval startTime, const struct timeval endTime);

#ifdef DEBUG
extern bool emu_verbose;
#endif

//#define ASM_PRECOMPILE_CACHE

#ifdef ASM_PRECOMPILE_CACHE
void precompile_cache_store_init(void);
void precompile_cache_load_init(void);
void precompile_cache_cleanup(void);
#endif

//#define ASM_CALL_METRICS

#ifdef ASM_CALL_METRICS

#include <x86intrin.h>
#include <linux/perf_event.h>

// Precompiles called by the generated assembly through an _opcode_*() handler
typedef enum {
    ASM_CALL_KECCAK = 0,
    ASM_CALL_SHA256,
    ASM_CALL_BLAKE2B,
    ASM_CALL_BLAKE3,
    ASM_CALL_BLAKE2S,
    ASM_CALL_POSEIDON2,
    ASM_CALL_POSEIDON1,
    ASM_CALL_ARITH256,
    ASM_CALL_ARITH256_MOD,
    ASM_CALL_ARITH384_MOD,
    ASM_CALL_SECP256K1_ADD,
    ASM_CALL_SECP256K1_DBL,
    ASM_CALL_SECP256R1_ADD,
    ASM_CALL_SECP256R1_DBL,
    ASM_CALL_FCALL,
    ASM_CALL_BN254_CURVE_ADD,
    ASM_CALL_BN254_CURVE_DBL,
    ASM_CALL_BN254_COMPLEX_ADD,
    ASM_CALL_BN254_COMPLEX_SUB,
    ASM_CALL_BN254_COMPLEX_MUL,
    ASM_CALL_BLS12_381_CURVE_ADD,
    ASM_CALL_BLS12_381_CURVE_DBL,
    ASM_CALL_BLS12_381_COMPLEX_ADD,
    ASM_CALL_BLS12_381_COMPLEX_SUB,
    ASM_CALL_BLS12_381_COMPLEX_MUL,
    ASM_CALL_BABYJUBJUB_ADD,
    ASM_CALL_ADD256,
    ASM_CALL_COUNT
} AsmCallId;

// Fcall function ids are small; unknown ids are accounted in slot 0
#define ASM_CALL_FCALL_IDS 32

// Accumulated raw cost of one precompile; the mean measurement overhead per call is subtracted
// when printing
typedef struct {
    uint64_t counter;
    uint64_t cycles;        // TSC cycles
    uint64_t instructions;  // Retired user-mode x86 instructions ("steps")
} AsmCallMetric;

typedef struct {
    AsmCallMetric call[ASM_CALL_COUNT];
    AsmCallMetric fcall[ASM_CALL_FCALL_IDS];
} AsmCallMetrics;

typedef struct {
    uint64_t cycles;
    uint64_t instructions;
} AsmCallSample;

extern AsmCallMetrics asm_call_metrics;
extern struct perf_event_mmap_page * asm_call_perf_page;

// Reads the retired instructions counter of this thread from user space, using the
// perf_event mmap page protocol; returns 0 if the counter is not available
static inline uint64_t asm_call_read_instructions (void)
{
    struct perf_event_mmap_page * pc = asm_call_perf_page;
    if (pc == NULL) return 0;
    uint32_t seq;
    uint64_t count;
    do {
        seq = pc->lock;
        __asm__ volatile ("" ::: "memory");
        uint32_t index = pc->index;
        count = pc->offset;
        if (pc->cap_user_rdpmc && (index != 0))
        {
            uint32_t lo, hi;
            __asm__ volatile ("rdpmc" : "=a"(lo), "=d"(hi) : "c"(index - 1));
            int64_t pmc = (int64_t)(((uint64_t)hi << 32) | lo);
            uint16_t shift = 64 - pc->pmc_width;
            pmc = (pmc << shift) >> shift;
            count += pmc;
        }
        __asm__ volatile ("" ::: "memory");
    } while (pc->lock != seq);
    return count;
}

static inline void asm_call_metrics_sample (AsmCallSample * sample)
{
    _mm_lfence();
    sample->cycles = __rdtsc();
    _mm_lfence();
    sample->instructions = asm_call_read_instructions();
    _mm_lfence();
}

void asm_call_metrics_record (AsmCallMetric * metric, const AsmCallSample * start);
void asm_call_metrics_record_fcall (uint64_t function_id, const AsmCallSample * start);
void reset_asm_call_metrics (void);
void print_asm_call_metrics (uint64_t total_duration);

#define ASM_CALL_METRICS_START() AsmCallSample asm_call_start; asm_call_metrics_sample(&asm_call_start)
#define ASM_CALL_METRICS_STOP(id) asm_call_metrics_record(&asm_call_metrics.call[id], &asm_call_start)
#define ASM_CALL_METRICS_STOP_FCALL(function_id) asm_call_metrics_record_fcall(function_id, &asm_call_start)

#else

#define ASM_CALL_METRICS_START()
#define ASM_CALL_METRICS_STOP(id)
#define ASM_CALL_METRICS_STOP_FCALL(function_id)

#endif // ASM_CALL_METRICS

#endif // EMU_ASM_HPP