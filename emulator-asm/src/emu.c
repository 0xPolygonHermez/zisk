
#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>
#include <sys/time.h>
#include <errno.h>
#include <unistd.h>
#include <stdlib.h>
#include <assert.h>
#include "emu.hpp"
#include "log.hpp"
#include "../../lib-c/c/src/bigint/add256.hpp"
#include "../../lib-c/c/src/ec/ec.hpp"
#include "../../lib-c/c/src/secp256r1/secp256r1.hpp"
#include "../../lib-c/c/src/fcall/fcall.hpp"
#include "../../lib-c/c/src/keccakf_cache/keccakf_cache.hpp"
#include "../../lib-c/c/src/arith256/arith256.hpp"
#include "../../lib-c/c/src/arith384/arith384.hpp"
#include "../../lib-c/c/src/bn254/bn254.hpp"
#include "../../lib-c/c/src/babyjubjub/babyjubjub.hpp"
#include "../../lib-c/c/src/bls12_381/bls12_381.hpp"
#include "../../lib-c/c/src/poseidon2/poseidon2_goldilocks.hpp"
#include "../../lib-c/c/src/poseidon1/poseidon1_goldilocks.hpp"
#include "../../lib-c/c/src/blake2/blake2.hpp"
#include "../../lib-c/c/src/blake3/blake3.hpp"
#include "../../lib-c/c/src/chfast/zisk_keccak.h"

extern void zisk_sha256(uint64_t state[4], uint64_t input[8]);

#ifdef DEBUG
bool emu_verbose = false;
#endif

#ifdef ASM_CALL_METRICS

#include <string.h>
#include <time.h>
#include <sys/mman.h>
#include <sys/syscall.h>

AsmCallMetrics asm_call_metrics;
struct perf_event_mmap_page * asm_call_perf_page = NULL;

static const char * asm_call_names[ASM_CALL_COUNT] = {
    [ASM_CALL_KECCAK] = "keccak",
    [ASM_CALL_SHA256] = "sha256",
    [ASM_CALL_BLAKE2B] = "blake2b",
    [ASM_CALL_BLAKE3] = "blake3",
    [ASM_CALL_BLAKE2S] = "blake2s",
    [ASM_CALL_POSEIDON2] = "poseidon2",
    [ASM_CALL_POSEIDON1] = "poseidon1",
    [ASM_CALL_ARITH256] = "arith256",
    [ASM_CALL_ARITH256_MOD] = "arith256_mod",
    [ASM_CALL_ARITH384_MOD] = "arith384_mod",
    [ASM_CALL_SECP256K1_ADD] = "secp256k1_add",
    [ASM_CALL_SECP256K1_DBL] = "secp256k1_dbl",
    [ASM_CALL_SECP256R1_ADD] = "secp256r1_add",
    [ASM_CALL_SECP256R1_DBL] = "secp256r1_dbl",
    [ASM_CALL_FCALL] = "fcall",
    [ASM_CALL_BN254_CURVE_ADD] = "bn254_curve_add",
    [ASM_CALL_BN254_CURVE_DBL] = "bn254_curve_dbl",
    [ASM_CALL_BN254_COMPLEX_ADD] = "bn254_complex_add",
    [ASM_CALL_BN254_COMPLEX_SUB] = "bn254_complex_sub",
    [ASM_CALL_BN254_COMPLEX_MUL] = "bn254_complex_mul",
    [ASM_CALL_BLS12_381_CURVE_ADD] = "bls12_381_curve_add",
    [ASM_CALL_BLS12_381_CURVE_DBL] = "bls12_381_curve_dbl",
    [ASM_CALL_BLS12_381_COMPLEX_ADD] = "bls12_381_complex_add",
    [ASM_CALL_BLS12_381_COMPLEX_SUB] = "bls12_381_complex_sub",
    [ASM_CALL_BLS12_381_COMPLEX_MUL] = "bls12_381_complex_mul",
    [ASM_CALL_BABYJUBJUB_ADD] = "babyjubjub_add",
    [ASM_CALL_ADD256] = "add256",
};

static const char * asm_call_fcall_names[ASM_CALL_FCALL_IDS] = {
    [0] = "unknown",
    [FCALL_SECP256K1_FP_INV_ID] = "secp256k1_fp_inv",
    [FCALL_SECP256K1_FN_INV_ID] = "secp256k1_fn_inv",
    [FCALL_SECP256K1_FP_SQRT_ID] = "secp256k1_fp_sqrt",
    [FCALL_SECP256K1_GLV_DECOMPOSE_ID] = "secp256k1_glv_decompose",
    [FCALL_SECP256R1_FN_INV_ID] = "secp256r1_fn_inv",
    [FCALL_BN254_FP_INV_ID] = "bn254_fp_inv",
    [FCALL_BN254_FP2_INV_ID] = "bn254_fp2_inv",
    [FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID] = "bn254_twist_add_line_coeffs",
    [FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID] = "bn254_twist_dbl_line_coeffs",
    [FCALL_BLS12_381_FP_INV_ID] = "bls12_381_fp_inv",
    [FCALL_BLS12_381_FP_SQRT_ID] = "bls12_381_fp_sqrt",
    [FCALL_BLS12_381_FP2_INV_ID] = "bls12_381_fp2_inv",
    [FCALL_BLS12_381_FP2_SQRT_ID] = "bls12_381_fp2_sqrt",
    [FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID] = "bls12_381_twist_add_line_coeffs",
    [FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID] = "bls12_381_twist_dbl_line_coeffs",
    [FCALL_BIN_DECOMP_ID] = "bin_decomp",
    [FCALL_MSB_POS_256_ID] = "msb_pos_256",
    [FCALL_MSB_POS_384_ID] = "msb_pos_384",
    [FCALL_UINT256_DIV_ID] = "uint256_div",
    [FCALL_UINT256_INV_ID] = "uint256_inv",
    [FCALL_UINT256_INV_MOD_ID] = "uint256_inv_mod",
    [FCALL_BIGINT_DIV_ID] = "bigint_div",
    [FCALL_SET_KECCAKF_CACHE_INDEX_ID] = "set_keccakf_cache_index",
    [FCALL_GET_KECCAKF_CACHE_INDEX_ID] = "get_keccakf_cache_index",
};

// Mean cost of an empty start/stop measurement, subtracted from the recorded calls when printing
static double asm_call_overhead_cycles = 0;
static double asm_call_overhead_instructions = 0;

// Run-level samples, to convert TSC cycles into time and to report the whole run
static AsmCallSample asm_call_run_start;
static struct timespec asm_call_run_start_time;

// Opens a user-mode retired instructions counter for the calling thread and maps its
// control page, so that asm_call_read_instructions() can use rdpmc
static void asm_call_perf_open (void)
{
    struct perf_event_attr attr;
    memset(&attr, 0, sizeof(attr));
    attr.type = PERF_TYPE_HARDWARE;
    attr.size = sizeof(attr);
    attr.config = PERF_COUNT_HW_INSTRUCTIONS;
    attr.exclude_kernel = 1;
    attr.exclude_hv = 1;
    int fd = syscall(SYS_perf_event_open, &attr, 0, -1, -1, 0);
    if (fd < 0)
    {
        asm_printf("ASM_CALL_METRICS: perf_event_open() failed errno=%d=%s; instructions will not be counted\n", errno, strerror(errno));
        return;
    }
    void * page = mmap(NULL, sysconf(_SC_PAGESIZE), PROT_READ, MAP_SHARED, fd, 0);
    if (page == MAP_FAILED)
    {
        asm_printf("ASM_CALL_METRICS: mmap() of perf page failed errno=%d=%s; instructions will not be counted\n", errno, strerror(errno));
        return;
    }
    asm_call_perf_page = (struct perf_event_mmap_page *)page;
    if (!asm_call_perf_page->cap_user_rdpmc)
    {
        asm_printf("ASM_CALL_METRICS: rdpmc is not allowed in user space; instructions will not be counted\n");
        asm_call_perf_page = NULL;
    }
}

static int asm_call_compare_u64 (const void * a, const void * b)
{
    uint64_t x = *(const uint64_t *)a, y = *(const uint64_t *)b;
    return (x < y) ? -1 : (x > y);
}

// Measures the mean cost of an empty start/stop pair. The mean, and not the minimum, is what has
// to be subtracted: the TSC advances in coarse steps on some CPUs (29 ticks on Zen 2), so single
// measurements are quantized and only their average is meaningful. The top 1% is left out, as
// interrupts. The CPU is kept busy for 100 ms before measuring, so that its clock has ramped up;
// measured on a cold CPU, the overhead comes out several times larger
static void asm_call_calibrate (void)
{
    struct timespec start, now;
    clock_gettime(CLOCK_MONOTONIC, &start);
    volatile uint64_t spin = 0;
    do
    {
        for (int i = 0; i < 100000; i++) spin++;
        clock_gettime(CLOCK_MONOTONIC, &now);
    } while ((uint64_t)(now.tv_sec - start.tv_sec) * 1000000000 + now.tv_nsec - start.tv_nsec < 100000000);

    const int n = 100000;
    static uint64_t cycles[100000], instructions[100000];
    for (int i = 0; i < n; i++)
    {
        AsmCallSample start, stop;
        asm_call_metrics_sample(&start);
        asm_call_metrics_sample(&stop);
        cycles[i] = stop.cycles - start.cycles;
        instructions[i] = stop.instructions - start.instructions;
    }
    qsort(cycles, n, sizeof(uint64_t), asm_call_compare_u64);
    qsort(instructions, n, sizeof(uint64_t), asm_call_compare_u64);
    const int kept = n - n / 100;
    double sum_cycles = 0, sum_instructions = 0;
    for (int i = 0; i < kept; i++)
    {
        sum_cycles += (double)cycles[i];
        sum_instructions += (double)instructions[i];
    }
    asm_call_overhead_cycles = sum_cycles / kept;
    asm_call_overhead_instructions = sum_instructions / kept;
}

// Accumulates the raw measurement; the overhead is subtracted when printing
static inline void asm_call_metric_add (AsmCallMetric * metric, const AsmCallSample * start, const AsmCallSample * stop)
{
    metric->counter++;
    metric->cycles += stop->cycles - start->cycles;
    metric->instructions += stop->instructions - start->instructions;
}

void asm_call_metrics_record (AsmCallMetric * metric, const AsmCallSample * start)
{
    AsmCallSample stop;
    asm_call_metrics_sample(&stop);
    asm_call_metric_add(metric, start, &stop);
}

void asm_call_metrics_record_fcall (uint64_t function_id, const AsmCallSample * start)
{
    AsmCallSample stop;
    asm_call_metrics_sample(&stop);
    asm_call_metric_add(&asm_call_metrics.call[ASM_CALL_FCALL], start, &stop);
    uint64_t slot = (function_id < ASM_CALL_FCALL_IDS) && (asm_call_fcall_names[function_id] != NULL) ? function_id : 0;
    asm_call_metric_add(&asm_call_metrics.fcall[slot], start, &stop);
}

void reset_asm_call_metrics (void)
{
    // Calibrate once: the emulation always runs with the CPU clock ramped up, as right after the
    // calibration warm-up, while a later calibration could find it slowed down by an idle wait
    static bool initialized = false;
    if (!initialized)
    {
        asm_call_perf_open();
        asm_call_calibrate();
        initialized = true;
    }
    memset(&asm_call_metrics, 0, sizeof(asm_call_metrics));
    clock_gettime(CLOCK_MONOTONIC, &asm_call_run_start_time);
    asm_call_metrics_sample(&asm_call_run_start);
}

// Prints one metric; for precompile calls, subtract_overhead removes the measurement overhead
// of each call from the raw totals
static void print_asm_call_metric (FILE * csv, uint64_t run_index, const char * kind, const char * name, const AsmCallMetric * metric, double ns_per_cycle, uint64_t run_cycles, bool subtract_overhead)
{
    if (metric->counter == 0) return;
    double cycles = (double)metric->cycles;
    double instructions = (double)metric->instructions;
    if (subtract_overhead)
    {
        cycles -= (double)metric->counter * asm_call_overhead_cycles;
        instructions -= (double)metric->counter * asm_call_overhead_instructions;
        if (cycles < 0) cycles = 0;
        if (instructions < 0) instructions = 0;
    }
    asm_printf("%-34s %10lu %10.3f %6.2f%% %10.1f %10.1f %10.1f\n",
        name,
        metric->counter,
        cycles * ns_per_cycle / 1000000.0,
        run_cycles == 0 ? 0.0 : cycles * 100.0 / (double)run_cycles,
        cycles * ns_per_cycle / (double)metric->counter,
        cycles / (double)metric->counter,
        instructions / (double)metric->counter);
    if (csv != NULL)
    {
        fprintf(csv, "%lu,%s,%s,%lu,%lu,%lu\n", run_index, kind, name, metric->counter, (uint64_t)(cycles + 0.5), (uint64_t)(instructions + 0.5));
    }
}

// Prints the accumulated precompile costs; if the ZISK_ASM_CALL_METRICS_CSV environment
// variable is set, they are also appended to that file as run,kind,name,calls,cycles,instructions,
// where run counts the emulations done by this process
void print_asm_call_metrics (uint64_t total_duration)
{
    static uint64_t run_index = 0;

    AsmCallSample run_stop;
    struct timespec run_stop_time;
    asm_call_metrics_sample(&run_stop);
    clock_gettime(CLOCK_MONOTONIC, &run_stop_time);
    uint64_t run_cycles = run_stop.cycles - asm_call_run_start.cycles;
    uint64_t run_ns = (uint64_t)(run_stop_time.tv_sec - asm_call_run_start_time.tv_sec) * 1000000000 + run_stop_time.tv_nsec - asm_call_run_start_time.tv_nsec;
    double ns_per_cycle = run_cycles == 0 ? 0.0 : (double)run_ns / (double)run_cycles;

    FILE * csv = NULL;
    const char * csv_path = getenv("ZISK_ASM_CALL_METRICS_CSV");
    if (csv_path != NULL)
    {
        csv = fopen(csv_path, "a");
        if (csv == NULL) asm_printf("ASM_CALL_METRICS: failed opening %s errno=%d=%s\n", csv_path, errno, strerror(errno));
    }

    asm_printf("\nprint_asm_call_metrics: emulation = %lu us, measurement overhead = %.1f cycles / %.1f instructions per call (mean, subtracted), %.3f ns per TSC cycle\n",
        total_duration, asm_call_overhead_cycles, asm_call_overhead_instructions, ns_per_cycle);
    asm_printf("%-34s %10s %10s %7s %10s %10s %10s\n", "precompile", "calls", "total ms", "% run", "ns/call", "cyc/call", "instr/call");

    AsmCallMetric total;
    memset(&total, 0, sizeof(total));
    for (int i = 0; i < ASM_CALL_COUNT; i++)
    {
        print_asm_call_metric(csv, run_index, "call", asm_call_names[i], &asm_call_metrics.call[i], ns_per_cycle, run_cycles, true);
        total.counter += asm_call_metrics.call[i].counter;
        total.cycles += asm_call_metrics.call[i].cycles;
        total.instructions += asm_call_metrics.call[i].instructions;
    }
    print_asm_call_metric(csv, run_index, "total", "TOTAL precompiles", &total, ns_per_cycle, run_cycles, true);

    AsmCallMetric run = { 1, run_cycles, run_stop.instructions - asm_call_run_start.instructions };
    print_asm_call_metric(csv, run_index, "run", "RUN (whole emulation)", &run, ns_per_cycle, run_cycles, false);

    if (asm_call_metrics.call[ASM_CALL_FCALL].counter != 0)
    {
        asm_printf("\n%-34s %10s %10s %7s %10s %10s %10s\n", "fcall function", "calls", "total ms", "% run", "ns/call", "cyc/call", "instr/call");
        for (int i = 0; i < ASM_CALL_FCALL_IDS; i++)
        {
            char name[64];
            snprintf(name, sizeof(name), "%2d %s", i, asm_call_fcall_names[i] == NULL ? "?" : asm_call_fcall_names[i]);
            print_asm_call_metric(csv, run_index, "fcall", name, &asm_call_metrics.fcall[i], ns_per_cycle, run_cycles, true);
        }
    }
    asm_printf("\n");

    if (csv != NULL) fclose(csv);
    run_index++;
}

#endif

#ifdef ASM_PRECOMPILE_CACHE

const char *precompile_cache_filename = "precompile_cache.bin";

FILE * precompile_file = NULL;
bool precompile_cache_storing = false;
bool precompile_cache_loading = false;

void precompile_cache_store_init(void)
{
    assert(precompile_file == NULL);
    assert(precompile_cache_storing == false);
    assert(precompile_cache_loading == false);
    precompile_file = fopen(precompile_cache_filename, "wb");
    if (precompile_file == NULL) {
        asm_printf("precompile_cache_store_init() Error opening file %s\n", precompile_cache_filename);
        exit(-1);
    }
    precompile_cache_storing = true;
}

void precompile_cache_load_init(void)
{
    assert(precompile_file == NULL);
    assert(precompile_cache_storing == false);
    assert(precompile_cache_loading == false);
    precompile_file = fopen(precompile_cache_filename, "rb");
    if (precompile_file == NULL) {
        asm_printf("precompile_cache_load_init() Error opening file %s\n", precompile_cache_filename);
        exit(-1);
    }
    precompile_cache_loading = true;
}

void precompile_cache_cleanup(void)
{
    assert(precompile_file != NULL);
    fclose(precompile_file);
    precompile_file = NULL;
    precompile_cache_storing = false;
    precompile_cache_loading = false;
}

// #define ASM_PRECOMPILE_CACHE_DEBUG
#ifdef ASM_PRECOMPILE_CACHE_DEBUG
uint64_t total_precompile_cache_size = 0;
uint64_t total_precompile_cache_counter = 0;
#endif
void precompile_cache_store( uint8_t* data, uint64_t size)
{
    assert(precompile_file != NULL);
    assert(precompile_cache_storing == true);
    fwrite(data, 1, size, precompile_file);
    fflush(precompile_file);
#ifdef ASM_PRECOMPILE_CACHE_DEBUG
    uint64_t previous_total_precompile_cache_size = total_precompile_cache_size;
    total_precompile_cache_size += size;
    total_precompile_cache_counter++;
    asm_printf("precompile_cache_store() Stored %lu bytes at pos=%lu file_size=%ld total_precompile_cache_size=%lu total_precompile_cache_counter=%lu\n", size, previous_total_precompile_cache_size, ftell(precompile_file), total_precompile_cache_size, total_precompile_cache_counter);
#endif
}

void precompile_cache_load( uint8_t* data, uint64_t size)
{
    assert(precompile_file != NULL);
    assert(precompile_cache_loading == true);
    size_t read_size = fread(data, 1, size, precompile_file);
    if (read_size != size) {
        asm_printf("precompile_cache_load() Error reading file %s read_size=%zu expected size=%lu pos=%ld\n", precompile_cache_filename, read_size, size, ftell(precompile_file));
        exit(-1);
    }
#ifdef ASM_PRECOMPILE_CACHE_DEBUG
    total_precompile_cache_size += size;
    total_precompile_cache_counter++;
    asm_printf("precompile_cache_load() Loaded %lu bytes at pos=%ld total_precompile_cache_size=%lu total_precompile_cache_counter=%lu\n", size, ftell(precompile_file), total_precompile_cache_size, total_precompile_cache_counter);
#endif
}

#endif

uint64_t print_abcflag_counter = 0;

extern int _print_abcflag(uint64_t a, uint64_t b, uint64_t c, uint64_t flag)
{
    uint64_t * pMem = (uint64_t *)0xa0012118;
    asm_printf("counter=%lu a=%08lx b=%08lx c=%08lx flag=%08lx mem=%08lx\n", print_abcflag_counter, a, b, c, flag, *pMem);
    // uint64_t *pRegs = (uint64_t *)RAM_ADDR;
    // for (int i=0; i<32; i++)
    // {
    //     asm_raw_printf("r%d=%08lx ", i, pRegs[i]);
    // }
    // asm_raw_printf("\n");
    // fflush(stdout);
    print_abcflag_counter++;
    return 0;
}

uint64_t printed_chars_counter = 0;

extern int _print_char(uint64_t param)
{
    printed_chars_counter++;
    char c = param;
    asm_raw_printf("%c", c);
    return 0;
}

uint64_t print_step_counter = 0;
extern int _print_step(uint64_t step)
{
#ifdef DEBUG
    asm_printf("step=%lu\n", print_step_counter);
    print_step_counter++;
    // struct timeval stop_time;
    // gettimeofday(&stop_time,NULL);
    // uint64_t duration = TimeDiff(start_time, stop_time);
    // uint64_t duration_s = duration/1000;
    // if (duration_s == 0) duration_s = 1;
    // uint64_t speed = step / duration_s;
    // if (emu_verbose) asm_printf("print_step() Counter=%d Step=%d Duration=%dus Speed=%dsteps/ms\n", print_step_counter, step, duration, speed);
#endif
    return 0;
}

uint64_t TimeDiff(const struct timeval startTime, const struct timeval endTime)
{
    struct timeval diff;

    // Calculate the time difference
    diff.tv_sec = endTime.tv_sec - startTime.tv_sec;
    if (endTime.tv_usec >= startTime.tv_usec)
    {
        diff.tv_usec = endTime.tv_usec - startTime.tv_usec;
    }
    else if (diff.tv_sec > 0)
    {
        diff.tv_usec = 1000000 + endTime.tv_usec - startTime.tv_usec;
        diff.tv_sec--;
    }
    else
    {
        // gettimeofday() can go backwards under some circumstances: NTP, multithread...
        //cerr << "Error: TimeDiff() got startTime > endTime: startTime.tv_sec=" << startTime.tv_sec << " startTime.tv_usec=" << startTime.tv_usec << " endTime.tv_sec=" << endTime.tv_sec << " endTime.tv_usec=" << endTime.tv_usec << endl;
        return 0;
    }

    // Return the total number of us
    return diff.tv_usec + 1000000 * diff.tv_sec;
}

extern int _opcode_keccak(uint64_t address)
{
    ASM_CALL_METRICS_START();
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) asm_printf("opcode_keccak() calling zisk_keccakf1600() counter=%lu address=%08lx\n", asm_call_metrics.call[ASM_CALL_KECCAK].counter, address);
#else
    if (emu_verbose)
    {
        asm_printf("opcode_keccak() calling zisk_keccakf1600() address=%08lx\n", address);
        for (uint64_t i=0; i<200; i++)
        {
            asm_raw_printf("%02x", ((uint8_t *)(uintptr_t)address)[i]);
        }
        asm_raw_printf("\n");
    }
#endif
#endif

    // Cache the input state if fcall_set_keccakf_cache_index() asked for it, before the
    // permutation overwrites it
    keccakf_cache_on_keccakf((const uint64_t *)address);

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call keccak-f compression function
        zisk_keccakf1600((uint64_t *)address);

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)address, 25*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)address, 25*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("opcode_keccak() called zisk_keccakf1600()\n");
        for (uint64_t i=0; i<200; i++)
        {
            asm_raw_printf("%02x", ((uint8_t *)(uintptr_t)address)[i]);
        }
        asm_raw_printf("\n");
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_KECCAK);
    return 0;
}

extern int _opcode_sha256(uint64_t * address)
{
    ASM_CALL_METRICS_START();
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) asm_printf("opcode_sha256() calling zisk_sha256() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_SHA256].counter, address);
#else
    if (emu_verbose) asm_printf("opcode_sha256() calling zisk_sha256() address=%p\n", address);
#endif
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call SHA256 compression function
        zisk_sha256((uint64_t *)address[0], (uint64_t *)address[1]);

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)address[0], 4*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)address[0], 4*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_sha256() called zisk_sha256()\n");
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_SHA256);
    return 0;
}

extern int _opcode_blake2b(uint64_t * address)
{
    ASM_CALL_METRICS_START();
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) asm_printf("opcode_blake2b() calling blake2b() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_BLAKE2B].counter, address);
#else
    if (emu_verbose) asm_printf("opcode_blake2b() calling blake2b() address=%p\n", address);
#endif
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call blake2b compression function
        blake2b_round((uint64_t *)address[1], (uint64_t *)address[2], address[0]);

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)address[1], 16*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)address[1], 16*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_blake2b() called blake2b()\n");
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BLAKE2B);
    return 0;
}

extern int _opcode_blake3(uint64_t * address)
{
    ASM_CALL_METRICS_START();
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) asm_printf("opcode_blake3() calling blake3_f() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_BLAKE3].counter, address);
#else
    if (emu_verbose) asm_printf("opcode_blake3() calling blake3_f() address=%p\n", address);
#endif
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call blake3 permutation function (address[0] = state ptr, address[1] = input ptr)
        blake3_f((uint32_t *)address[0], (const uint32_t *)address[1]);

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)address[0], 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)address[0], 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_blake3() called blake3_f()\n");
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BLAKE3);
    return 0;
}

extern int _opcode_blake2s(uint64_t * address)
{
    ASM_CALL_METRICS_START();
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) asm_printf("opcode_blake2s() calling blake2s_f() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_BLAKE2S].counter, address);
#else
    if (emu_verbose) asm_printf("opcode_blake2s() calling blake2s_f() address=%p\n", address);
#endif
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call blake2s permutation function (address[0] = state ptr, address[1] = input ptr)
        blake2s_f((uint32_t *)address[0], (const uint32_t *)address[1]);

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)address[0], 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)address[0], 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_blake2s() called blake2s_f()\n");
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BLAKE2S);
    return 0;
}

extern int _opcode_poseidon2(uint64_t address)
{
    ASM_CALL_METRICS_START();
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) asm_printf("opcode_poseidon2() calling poseidon2_hash() counter=%lu address=%08lx\n", asm_call_metrics.call[ASM_CALL_POSEIDON2].counter, address);
#else
    if (emu_verbose) asm_printf("opcode_poseidon2() calling poseidon2_hash() address=%08lx\n", address);
#endif
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call poseidon2 compression function
        poseidon2_hash((uint64_t *)address);

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)address, 16*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)address, 16*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_poseidon2() called poseidon2_hash()\n");
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_POSEIDON2);
    return 0;
}

extern int _opcode_poseidon1(uint64_t address)
{
    ASM_CALL_METRICS_START();
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) asm_printf("opcode_poseidon1() calling poseidon1_hash() counter=%lu address=%08lx\n", asm_call_metrics.call[ASM_CALL_POSEIDON1].counter, address);
#else
    if (emu_verbose) asm_printf("opcode_poseidon1() calling poseidon1_hash() address=%08lx\n", address);
#endif
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call poseidon1 compression function
        poseidon1_hash((uint64_t *)address);

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)address, 16*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)address, 16*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_poseidon1() called poseidon1_hash()\n");
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_POSEIDON1);
    return 0;
}

extern int _opcode_arith256(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    // Call arithmetic 256 operation
    uint64_t * a = (uint64_t *)address[0];
    uint64_t * b = (uint64_t *)address[1];
    uint64_t * c = (uint64_t *)address[2];
    uint64_t * dl = (uint64_t *)address[3];
    uint64_t * dh = (uint64_t *)address[4];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("opcode_arith256() calling Arith256() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_ARITH256].counter, address);
#else
        asm_printf("opcode_arith256() calling Arith256() address=%p\n", address);
#endif
        asm_printf("a = %lx:%lx:%lx:%lx\n", a[3], a[2], a[1], a[0]);
        asm_printf("b = %lx:%lx:%lx:%lx\n", b[3], b[2], b[1], b[0]);
        asm_printf("c = %lx:%lx:%lx:%lx\n", c[3], c[2], c[1], c[0]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call arithmetic 256 operation
        int result = Arith256 (a, b, c, dl, dh);
        if (result != 0)
        {
            asm_printf("_opcode_arith256_add() failed calling Arith256() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)dl, 4*8);
        precompile_cache_store((uint8_t *)dh, 4*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)dl, 4*8);
        precompile_cache_load((uint8_t *)dh, 4*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_arith256() called Arith256()\n");
    if (emu_verbose)
    {
        asm_printf("dl = %lx:%lx:%lx:%lx\n", dl[3], dl[2], dl[1], dl[0]);
        asm_printf("dh = %lx:%lx:%lx:%lx\n", dh[3], dh[2], dh[1], dh[0]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_ARITH256);
    return 0;
}

// Fast assembly implementation of (a*b + c) mod module (emulator-asm/src/arith_eq/arith256_mod.asm).
// Takes the same 5-pointer struct as this opcode; used in the compute (no-hints) path below.
extern int arith256_mod(uint64_t * address);

extern int _opcode_arith256_mod(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    // Call arithmetic 256 module operation
    uint64_t * a = (uint64_t *)address[0];
    uint64_t * b = (uint64_t *)address[1];
    uint64_t * c = (uint64_t *)address[2];
    uint64_t * module = (uint64_t *)address[3];
    uint64_t * d = (uint64_t *)address[4];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("opcode_arith256_mod() calling arith256_mod() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_ARITH256_MOD].counter, address);
#else
        asm_printf("opcode_arith256_mod() calling arith256_mod() address=%p\n", address);
#endif
        asm_printf("a = %lx:%lx:%lx:%lx\n", a[3], a[2], a[1], a[0]);
        asm_printf("b = %lx:%lx:%lx:%lx\n", b[3], b[2], b[1], b[0]);
        asm_printf("c = %lx:%lx:%lx:%lx\n", c[3], c[2], c[1], c[0]);
        asm_printf("module = %lx:%lx:%lx:%lx\n", module[3], module[2], module[1], module[0]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Compute (no-hints path): Montgomery fast path for the usual moduli, otherwise the
        // assembly long division implementation instead of the Rust Arith256Mod.
        if (Arith256ModFast(a, b, c, module, d) != 0)
        {
            int result = arith256_mod (address);
            if (result != 0)
            {
                asm_printf("_opcode_arith256_mod() failed calling arith256_mod() result=%d;", result);
                exit(-1);
            }
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)d, 4*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)d, 4*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_arith256_mod() called arith256_mod()\n");
    if (emu_verbose)
    {
        asm_printf("d = %lx:%lx:%lx:%lx\n", d[3], d[2], d[1], d[0]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_ARITH256_MOD);
    return 0;
}

extern int _opcode_arith384_mod(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    // Call arithmetic 256 module operation
    uint64_t * a = (uint64_t *)address[0];
    uint64_t * b = (uint64_t *)address[1];
    uint64_t * c = (uint64_t *)address[2];
    uint64_t * module = (uint64_t *)address[3];
    uint64_t * d = (uint64_t *)address[4];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("opcode_arith384_mod() calling Arith384Mod() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_ARITH384_MOD].counter, address);
#else
        asm_printf("opcode_arith384_mod() calling Arith384Mod() address=%p\n", address);
#endif
        asm_printf("a = %lx:%lx:%lx:%lx:%lx:%lx\n", a[5], a[4], a[3], a[2], a[1], a[0]);
        asm_printf("b = %lx:%lx:%lx:%lx:%lx:%lx\n", b[5], b[4], b[3], b[2], b[1], b[0]);
        asm_printf("c = %lx:%lx:%lx:%lx:%lx:%lx\n", c[5], c[4], c[3], c[2], c[1], c[0]);
        asm_printf("module = %lx:%lx:%lx:%lx:%lx:%lx\n", module[5], module[4], module[3], module[2], module[1], module[0]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call arithmetic 384 module operation
        int result = Arith384Mod (a, b, c, module, d);
        if (result != 0)
        {
            asm_printf("_opcode_arith384_mod() failed calling Arith384Mod() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)d, 6*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)d, 6*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_arith384_mod() called Arith384Mod()\n");
    if (emu_verbose)
    {
        asm_printf("d = %lx:%lx:%lx:%lx:%lx:%lx\n", d[5], d[4], d[3], d[2], d[1], d[0]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_ARITH384_MOD);
    return 0;
}

extern int _opcode_secp256k1_add(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("opcode_secp256k1_add() calling AddPointEcP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_SECP256K1_ADD].counter, address, p1, p2);
#else
        asm_printf("opcode_secp256k1_add() calling AddPointEcP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call point addition function
        int result = AddPointEcP (
            0,
            p1, // p1 = [x1, y1] = 8x64bits
            p2, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_secp256k1_add() failed calling AddPointEcP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p3.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p3.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_SECP256K1_ADD);
    return 0;
}

extern int _opcode_secp256k1_dbl(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = address;

#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("opcode_secp256k1_dbl() calling AddPointEcP() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_SECP256K1_DBL].counter, address);
#else
        asm_printf("opcode_secp256k1_dbl() calling AddPointEcP() address=%p\n", address);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        int result = AddPointEcP (
            1,
            p1, // p1 = [x1, y1] = 8x64bits
            NULL, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_secp256k1_dbl() failed calling AddPointEcP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_secp256k1_dbl() called AddPointEcP()\n");
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_SECP256K1_DBL);
    return 0;
}

extern int _opcode_secp256r1_add(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("opcode_secp256r1_add() calling AddPointEcP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_SECP256R1_ADD].counter, address, p1, p2);
#else
        asm_printf("opcode_secp256r1_add() calling AddPointEcP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
        asm_printf("p2.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0], p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4], p2[7], p2[6], p2[5], p2[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call point addition function
        int result = secp256r1_add_point_ecp (
            0,
            p1, // p1 = [x1, y1] = 8x64bits
            p2, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_secp256r1_add() failed calling AddPointEcP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p3 = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_SECP256R1_ADD);
    return 0;
}

extern int _opcode_secp256r1_dbl(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = address;

#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("opcode_secp256r1_dbl() calling AddPointEcP() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_SECP256R1_DBL].counter, address);
#else
        asm_printf("opcode_secp256r1_dbl() calling AddPointEcP() address=%p\n", address);
#endif
        asm_printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        int result = secp256r1_add_point_ecp (
            1,
            p1, // p1 = [x1, y1] = 8x64bits
            NULL, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_secp256r1_dbl() failed calling secp256r1_add_point_ecp() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_secp256r1_dbl() called secp256r1_add_point_ecp()\n");
    if (emu_verbose)
    {
        asm_printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_SECP256R1_DBL);
    return 0;
}

extern uint64_t MEM_TRACE_ADDRESS;
extern uint64_t fcall_ctx;
uint64_t print_fcall_ctx_counter = 0;

extern int _print_fcall_ctx(void)
{
    struct FcallContext * ctx = (struct FcallContext *)&fcall_ctx;
    asm_printf("print_fcall_ctx(%lu) address=0x%p\n", print_fcall_ctx_counter, ctx);
    asm_printf("\tfunction_id=0x%lu\n", ctx->function_id);
    asm_printf("\tparams_max_size=%lu=0x%lx\n", ctx->params_max_size, ctx->params_max_size);
    asm_printf("\tparams_size=0x%lu\n", ctx->params_size);
    for (int i=0; i<32; i++)
    {
        asm_printf("\t\tparams[%d]=%lu=0x%lx\n", i, ctx->params[i], ctx->params[i]);
    }
    asm_printf("\tresult_max_size=0x%lu\n", ctx->result_max_size);
    asm_printf("\tresult_size=0x%lu\n", ctx->result_size);
    for (int i=0; i<32; i++)
    {
        asm_printf("\t\tresult[%d]=%lu=0x%lx\n", i, ctx->result[i], ctx->result[i]);
    }
    asm_printf("\n");
    print_fcall_ctx_counter++;
}

extern int _opcode_fcall(struct FcallContext * ctx)
{
    ASM_CALL_METRICS_START();
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) asm_printf("_opcode_fcall(%lu) counter=%lu\n", ctx->function_id, asm_call_metrics.call[ASM_CALL_FCALL].counter);
#else
    if (emu_verbose) asm_printf("_opcode_fcall(%lu)\n", ctx->function_id);
#endif
    if (emu_verbose)
    {
        asm_printf("_opcode_fcall() calling Fcall() with params_size=%lu\n", ctx->params_size);
        asm_raw_printf("params=");
        for (uint64_t i=0; i<ctx->params_size; i++)
        {
            asm_raw_printf("%lx ", ctx->params[i]);
        }
        asm_raw_printf("\n");
    }
#endif
    int iresult;

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call fcall
        iresult = Fcall(ctx);
        if (iresult < 0)
        {
            asm_printf("_opcode_fcall() failed calling Fcall() result=%d\n", iresult);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)&ctx->result_size, 1*8);
        precompile_cache_store((uint8_t *)&ctx->result, ctx->result_size*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)&ctx->result_size, 1*8);
        precompile_cache_load((uint8_t *)&ctx->result, ctx->result_size*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("_opcode_fcall() called Fcall() and got result_size=%lu\n", ctx->result_size);
        asm_raw_printf("results=");
        for (uint64_t i=0; i<ctx->result_size; i++)
        {
            asm_raw_printf("%lx ", ctx->result[i]);
        }
        asm_raw_printf("\n");
    }
#endif

    ASM_CALL_METRICS_STOP_FCALL(ctx->function_id);
    return iresult;
}

/*********/
/* BN254 */
/*********/

extern int _opcode_bn254_curve_add(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bn254_curve_add() calling BN254CurveAddP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BN254_CURVE_ADD].counter, address, p1, p2);
#else
        asm_printf("_opcode_bn254_curve_add() calling BN254CurveAddP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call point addition function
        int result = BN254CurveAddP (
            p1, // p1 = [x1, y1] = 8x64bits
            p2, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bn254_curve_add() failed calling BN254CurveAddP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BN254_CURVE_ADD);
    return 0;
}

extern int _opcode_babyjubjub_add(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_babyjubjub_add() calling BabyJubJubAddP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BABYJUBJUB_ADD].counter, address, p1, p2);
#else
        asm_printf("_opcode_babyjubjub_add() calling BabyJubJubAddP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call point addition function
        int result = BabyJubJubAddP (
            p1, // p1 = [x1, y1] = 8x64bits
            p2, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_babyjubjub_add() failed calling BabyJubJubAddP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BABYJUBJUB_ADD);
    return 0;
}

extern int _opcode_bn254_curve_dbl(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = address;
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bn254_curve_dbl() calling BN254CurveDblP() counter=%lu address=%p p1_address=%p\n", asm_call_metrics.call[ASM_CALL_BN254_CURVE_DBL].counter, address, p1);
#else
        asm_printf("_opcode_bn254_curve_dbl() calling BN254CurveDblP() address=%p p1_address=%p\n", address, p1);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call point doubling function
        int result = BN254CurveDblP (
            p1, // p1 = [x1, y1] = 8x64bits
            p1 // p1 = [x1, y1] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bn254_curve_dbl() failed calling BN254CurveDblP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BN254_CURVE_DBL);
    return 0;
}

extern int _opcode_bn254_complex_add(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bn254_complex_add() calling BN254ComplexAddP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BN254_COMPLEX_ADD].counter, address, p1, p2);
#else
        asm_printf("_opcode_bn254_complex_add() calling BN254ComplexAddP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call complex addition function
        int result = BN254ComplexAddP (
            p1, // p1 = [x1, y1] = 8x64bits
            p2, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bn254_complex_add() failed calling BN254ComplexAddP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BN254_COMPLEX_ADD);
    return 0;
}

extern int _opcode_bn254_complex_sub(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bn254_complex_sub() calling BN254ComplexSubP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BN254_COMPLEX_SUB].counter, address, p1, p2);
#else
        asm_printf("_opcode_bn254_complex_sub() calling BN254ComplexSubP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call complex subtraction function
        int result = BN254ComplexSubP (
            p1, // p1 = [x1, y1] = 8x64bits
            p2, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bn254_complex_sub() failed calling BN254ComplexSubP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BN254_COMPLEX_SUB);
    return 0;
}

extern int _opcode_bn254_complex_mul(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bn254_complex_mul() calling BN254ComplexMulP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BN254_COMPLEX_MUL].counter, address, p1, p2);
#else
        asm_printf("_opcode_bn254_complex_mul() calling BN254ComplexMulP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call complex multiplication function
        int result = BN254ComplexMulP (
            p1, // p1 = [x1, y1] = 8x64bits
            p2, // p2 = [x2, y2] = 8x64bits
            p1 // p3 = [x3, y3] = 8x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bn254_complex_mul() failed calling BN254ComplexMulP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 8*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 8*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BN254_COMPLEX_MUL);
    return 0;
}

/*************/
/* BLS12_381 */
/*************/

extern int _opcode_bls12_381_curve_add(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bls12_381_curve_add() calling BLS12_381CurveAddP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BL12_381_CURVE_ADD].counter, address, p1, p2);
#else
        asm_printf("_opcode_bls12_381_curve_add() calling BLS12_381CurveAddP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p2[5], p2[4], p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p2[11], p2[10], p2[9], p2[8], p2[7], p2[6]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call point addition function
        int result = BLS12_381CurveAddP (
            p1, // p1 = [x1, y1] = 12x64bits
            p2, // p2 = [x2, y2] = 12x64bits
            p1 // p3 = [x3, y3] = 12x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bls12_381_curve_add() failed calling BLS12_381CurveAddP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 12*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 12*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BLS12_381_CURVE_ADD);
    return 0;
}

extern int _opcode_bls12_381_curve_dbl(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = address;
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bls12_381_curve_dbl() calling BLS12_381CurveDblP() counter=%lu address=%p p1_address=%p\n", asm_call_metrics.call[ASM_CALL_BLS12_381_CURVE_DBL].counter, address, p1);
#else
        asm_printf("_opcode_bls12_381_curve_dbl() calling BLS12_381CurveDblP() address=%p p1_address=%p\n", address, p1);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call point doubling function
        int result = BLS12_381CurveDblP (
            p1, // p1 = [x1, y1] = 12x64bits
            p1 // p1 = [x1, y1] = 12x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bls12_381_curve_dbl() failed calling BLS12_381CurveDblP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 12*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 12*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BLS12_381_CURVE_DBL);
    return 0;
}

extern int _opcode_bls12_381_complex_add(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bls12_381_complex_add() calling BLS12_381ComplexAddP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BLS12_381_COMPLEX_ADD].counter, address, p1, p2);
#else
        asm_printf("_opcode_bls12_381_complex_add() calling BLS12_381ComplexAddP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p2[5], p2[4], p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p2[11], p2[10], p2[9], p2[8], p2[7], p2[6]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call complex addition function
        int result = BLS12_381ComplexAddP (
            p1, // p1 = [x1, y1] = 12x64bits
            p2, // p2 = [x2, y2] = 12x64bits
            p1 // p3 = [x3, y3] = 12x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bls12_381_complex_add() failed calling BLS12_381ComplexAddP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 12*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 12*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BLS12_381_COMPLEX_ADD);
    return 0;
}

extern int _opcode_bls12_381_complex_sub(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bls12_381_complex_sub() calling BLS12_381ComplexSubP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BLS12_381_COMPLEX_SUB].counter, address, p1, p2);
#else
        asm_printf("_opcode_bls12_381_complex_sub() calling BLS12_381ComplexSubP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p2[5], p2[4], p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p2[11], p2[10], p2[9], p2[8], p2[7], p2[6]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call complex subtraction function
        int result = BLS12_381ComplexSubP (
            p1, // p1 = [x1, y1] = 12x64bits
            p2, // p2 = [x2, y2] = 12x64bits
            p1 // p3 = [x3, y3] = 12x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bls12_381_complex_sub() failed calling BLS12_381ComplexSubP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 12*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 12*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BLS12_381_COMPLEX_SUB);
    return 0;
}

extern int _opcode_bls12_381_complex_mul(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("_opcode_bls12_381_complex_mul() calling BLS12_381ComplexMulP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.call[ASM_CALL_BLS12_381_COMPLEX_MUL].counter, address, p1, p2);
#else
        asm_printf("_opcode_bls12_381_complex_mul() calling BLS12_381ComplexMulP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
        asm_printf("p2.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p2[5], p2[4], p2[3], p2[2], p2[1], p2[0]);
        asm_printf("p2.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p2[11], p2[10], p2[9], p2[8], p2[7], p2[6]);
    }
#endif

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif
        // Call complex multiplication function
        int result = BLS12_381ComplexMulP (
            p1, // p1 = [x1, y1] = 12x64bits
            p2, // p2 = [x2, y2] = 12x64bits
            p1 // p3 = [x3, y3] = 12x64bits
        );
        if (result != 0)
        {
            asm_printf("_opcode_bls12_381_complex_mul() failed calling BLS12_381ComplexMulP() result=%d;", result);
            exit(-1);
        }

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)p1, 12*8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)p1, 12*8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose)
    {
        asm_printf("p1.x = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[5], p1[4], p1[3], p1[2], p1[1], p1[0]);
        asm_printf("p1.y = %lx:%lx:%lx:%lx:%lx:%lx\n", p1[11], p1[10], p1[9], p1[8], p1[7], p1[6]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_BLS12_381_COMPLEX_MUL);
    return 0;
}


extern uint64_t _opcode_add256(uint64_t * address)
{
    ASM_CALL_METRICS_START();

    // Call arithmetic 256 operation
    uint64_t * a = (uint64_t *)address[0];
    uint64_t * b = (uint64_t *)address[1];
    uint64_t cin = (uint64_t)address[2];
    uint64_t * c = (uint64_t *)address[3];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        asm_printf("opcode_add256() calling Add256() counter=%lu address=%p\n", asm_call_metrics.call[ASM_CALL_ADD256].counter, address);
#else
        asm_printf("opcode_add256() calling Add256() address=%p\n", address);
#endif
        asm_printf("a = %lx:%lx:%lx:%lx\n", a[3], a[2], a[1], a[0]);
        asm_printf("b = %lx:%lx:%lx:%lx\n", b[3], b[2], b[1], b[0]);
        asm_printf("c = %lx:%lx:%lx:%lx\n", c[3], c[2], c[1], c[0]);
    }
#endif

    uint64_t cout = 0;

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_storing)
    {
#endif

        // cout = [0,1] ok, cout < 0 error
        int icout = Add256 (a, b, cin, c);
        if (icout < 0)
        {
            asm_printf("_opcode_add256() failed calling Add256() cout=%d;", icout);
            exit(-1);
        }
        cout = (uint64_t)icout;

#ifdef ASM_PRECOMPILE_CACHE
        // Store result in cache
        precompile_cache_store((uint8_t *)c, 4*8);
        precompile_cache_store((uint8_t *)&cout, 8);
    }
    else if (precompile_cache_loading)
    {
        // Load result from cache
        precompile_cache_load((uint8_t *)c, 4*8);
        precompile_cache_load((uint8_t *)&cout, 8);
    }
#endif

#ifdef DEBUG
    if (emu_verbose) asm_printf("opcode_add256() called Add256()\n");
    if (emu_verbose)
    {
        asm_printf("cout = %lu\n", cout);
        asm_printf("c = %lx:%lx:%lx:%lx\n", c[3], c[2], c[1], c[0]);
    }
#endif
    ASM_CALL_METRICS_STOP(ASM_CALL_ADD256);
    return cout;
}
