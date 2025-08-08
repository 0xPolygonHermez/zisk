
#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>
#include <sys/time.h>
#include <errno.h>
#include <unistd.h>
#include <stdlib.h>
#include "emu.hpp"
#include "../../lib-c/c/src/ec/ec.hpp"
#include "../../lib-c/c/src/fcall/fcall.hpp"
#include "../../lib-c/c/src/arith256/arith256.hpp"
#include "../../lib-c/c/src/bn254/bn254.hpp"
#include "bcon/bcon_sha256.hpp"

extern void zisk_sha256(uint64_t state[4], uint64_t input[8]);

extern void keccakf1600_generic(uint64_t state[25]);

#ifdef DEBUG
bool emu_verbose = false;
#endif

#ifdef ASM_CALL_METRICS

AsmCallMetrics asm_call_metrics; 

struct timeval asm_call_start, asm_call_stop;

void reset_asm_call_metrics (void)
{
    asm_call_metrics.keccak_counter = 0;
    asm_call_metrics.keccak_duration = 0;
    asm_call_metrics.sha256_counter = 0;
    asm_call_metrics.sha256_duration = 0;
    asm_call_metrics.arith256_counter = 0;
    asm_call_metrics.arith256_duration = 0;
    asm_call_metrics.arith256_mod_counter = 0;
    asm_call_metrics.arith256_mod_duration = 0;
    asm_call_metrics.secp256k1_add_counter = 0;
    asm_call_metrics.secp256k1_add_duration = 0;
    asm_call_metrics.secp256k1_dbl_counter = 0;
    asm_call_metrics.secp256k1_dbl_duration = 0;
    asm_call_metrics.fcall_counter = 0;
    asm_call_metrics.fcall_duration = 0;
    asm_call_metrics.inverse_fp_ec_counter = 0;
    asm_call_metrics.inverse_fp_ec_duration = 0;
    asm_call_metrics.inverse_fn_ec_counter = 0;
    asm_call_metrics.inverse_fn_ec_duration = 0;
    asm_call_metrics.sqrt_fp_ec_parity_counter = 0;
    asm_call_metrics.sqrt_fp_ec_parity_duration = 0;
    asm_call_metrics.bn254_curve_add_counter = 0;
    asm_call_metrics.bn254_curve_add_duration = 0;
    asm_call_metrics.bn254_curve_dbl_counter = 0;
    asm_call_metrics.bn254_curve_dbl_duration = 0;
    asm_call_metrics.bn254_complex_add_counter = 0;
    asm_call_metrics.bn254_complex_add_duration = 0;
    asm_call_metrics.bn254_complex_sub_counter = 0;
    asm_call_metrics.bn254_complex_sub_duration = 0;
    asm_call_metrics.bn254_complex_mul_counter = 0;
    asm_call_metrics.bn254_complex_mul_duration = 0;
}

void print_asm_call_metrics (uint64_t total_duration)
{
    uint64_t duration, percentage, asm_call_total_duration = 0;

    printf("\nprint_asm_call_metrics:\n");

    // Print keccak metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.keccak_duration * 1000) / total_duration;
    duration = asm_call_metrics.keccak_counter == 0 ? 0 : (asm_call_metrics.keccak_duration * 1000) / asm_call_metrics.keccak_counter;
    asm_call_total_duration += asm_call_metrics.keccak_duration;
    printf("Keccak: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.keccak_counter,
        asm_call_metrics.keccak_duration,
        duration,
        percentage);

    // Print SHA256 metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.sha256_duration * 1000) / total_duration;
    duration = asm_call_metrics.sha256_counter == 0 ? 0 : (asm_call_metrics.sha256_duration * 1000) / asm_call_metrics.sha256_counter;
    asm_call_total_duration += asm_call_metrics.sha256_duration;
    printf("SHA256: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.sha256_counter,
        asm_call_metrics.sha256_duration,
        duration,
        percentage);

    // Print arith256 metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.arith256_duration * 1000) / total_duration;
    duration = asm_call_metrics.arith256_counter == 0 ? 0 : (asm_call_metrics.arith256_duration * 1000) / asm_call_metrics.arith256_counter;
    asm_call_total_duration += asm_call_metrics.arith256_duration;
    printf("Arith256: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.arith256_counter,
        asm_call_metrics.arith256_duration,
        duration,
        percentage);

    // Print arith256_mod metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.arith256_mod_duration * 1000) / total_duration;
    duration = asm_call_metrics.arith256_mod_counter == 0 ? 0 : (asm_call_metrics.arith256_mod_duration * 1000) / asm_call_metrics.arith256_mod_counter;
    asm_call_total_duration += asm_call_metrics.arith256_mod_duration;
    printf("Arith256 mod: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.arith256_mod_counter,
        asm_call_metrics.arith256_mod_duration,
        duration,
        percentage);

    // Print secp256k1_add metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.secp256k1_add_duration * 1000) / total_duration;
    duration = asm_call_metrics.secp256k1_add_counter == 0 ? 0 : (asm_call_metrics.secp256k1_add_duration * 1000) / asm_call_metrics.secp256k1_add_counter;
    asm_call_total_duration += asm_call_metrics.secp256k1_add_duration;
    printf("secp256k1_add: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.secp256k1_add_counter,
        asm_call_metrics.secp256k1_add_duration,
        duration,
        percentage);

    // Print secp256k1_dbl metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.secp256k1_dbl_duration * 1000) / total_duration;
    duration = asm_call_metrics.secp256k1_dbl_counter == 0 ? 0 : (asm_call_metrics.secp256k1_dbl_duration * 1000) / asm_call_metrics.secp256k1_dbl_counter;
    asm_call_total_duration += asm_call_metrics.secp256k1_dbl_duration;
    printf("secp256k1_dbl: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.secp256k1_dbl_counter,
        asm_call_metrics.secp256k1_dbl_duration,
        duration,
        percentage);

    // Print fcall metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.fcall_duration * 1000) / total_duration;
    duration = asm_call_metrics.fcall_counter == 0 ? 0 : (asm_call_metrics.fcall_duration * 1000) / asm_call_metrics.fcall_counter;
    asm_call_total_duration += asm_call_metrics.fcall_duration;
    printf("fcall: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.fcall_counter,
        asm_call_metrics.fcall_duration,
        duration,
        percentage);

    // Print inverse_fp_ec metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.inverse_fp_ec_duration * 1000) / total_duration;
    duration = asm_call_metrics.inverse_fp_ec_counter == 0 ? 0 : (asm_call_metrics.inverse_fp_ec_duration * 1000) / asm_call_metrics.inverse_fp_ec_counter;
    asm_call_total_duration += asm_call_metrics.inverse_fp_ec_duration;
    printf("inverse_fp_ec: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.inverse_fp_ec_counter,
        asm_call_metrics.inverse_fp_ec_duration,
        duration,
        percentage);

    // Print inverse_fn_ec metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.inverse_fn_ec_duration * 1000) / total_duration;
    duration = asm_call_metrics.inverse_fn_ec_counter == 0 ? 0 : (asm_call_metrics.inverse_fn_ec_duration * 1000) / asm_call_metrics.inverse_fn_ec_counter;
    asm_call_total_duration += asm_call_metrics.inverse_fn_ec_duration;
    printf("inverse_fn_ec: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.inverse_fn_ec_counter,
        asm_call_metrics.inverse_fn_ec_duration,
        duration,
        percentage);

    // Print sqrt_fp_ec_parity metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.sqrt_fp_ec_parity_duration * 1000) / total_duration;
    duration = asm_call_metrics.sqrt_fp_ec_parity_counter == 0 ? 0 : (asm_call_metrics.sqrt_fp_ec_parity_duration * 1000) / asm_call_metrics.sqrt_fp_ec_parity_counter;
    asm_call_total_duration += asm_call_metrics.sqrt_fp_ec_parity_duration;
    printf("sqrt_fp_ec_parity: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.sqrt_fp_ec_parity_counter,
        asm_call_metrics.sqrt_fp_ec_parity_duration,
        duration,
        percentage);

    // Print bn254_curve_add metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.bn254_curve_add_duration * 1000) / total_duration;
    duration = asm_call_metrics.bn254_curve_add_counter == 0 ? 0 : (asm_call_metrics.bn254_curve_add_duration * 1000) / asm_call_metrics.bn254_curve_add_counter;
    asm_call_total_duration += asm_call_metrics.bn254_curve_add_duration;
    printf("bn254_curve_add: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.bn254_curve_add_counter,
        asm_call_metrics.bn254_curve_add_duration,
        duration,
        percentage);

    // Print bn254_curve_dbl metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.bn254_curve_dbl_duration * 1000) / total_duration;
    duration = asm_call_metrics.bn254_curve_dbl_counter == 0 ? 0 : (asm_call_metrics.bn254_curve_dbl_duration * 1000) / asm_call_metrics.bn254_curve_dbl_counter;
    asm_call_total_duration += asm_call_metrics.bn254_curve_dbl_duration;
    printf("bn254_curve_dbl: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.bn254_curve_dbl_counter,
        asm_call_metrics.bn254_curve_dbl_duration,
        duration,
        percentage);

    // Print bn254_complex_add metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.bn254_complex_add_duration * 1000) / total_duration;
    duration = asm_call_metrics.bn254_complex_add_counter == 0 ? 0 : (asm_call_metrics.bn254_complex_add_duration * 1000) / asm_call_metrics.bn254_complex_add_counter;
    asm_call_total_duration += asm_call_metrics.bn254_complex_add_duration;
    printf("bn254_complex_add: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.bn254_complex_add_counter,
        asm_call_metrics.bn254_complex_add_duration,
        duration,
        percentage);

    // Print bn254_complex_sub metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.bn254_complex_sub_duration * 1000) / total_duration;
    duration = asm_call_metrics.bn254_complex_sub_counter == 0 ? 0 : (asm_call_metrics.bn254_complex_sub_duration * 1000) / asm_call_metrics.bn254_complex_sub_counter;
    asm_call_total_duration += asm_call_metrics.bn254_complex_sub_duration;
    printf("bn254_complex_sub: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.bn254_complex_sub_counter,
        asm_call_metrics.bn254_complex_sub_duration,
        duration,
        percentage);

    // Print bn254_complex_mul metrics
    percentage = total_duration == 0 ? 0 : (asm_call_metrics.bn254_complex_mul_duration * 1000) / total_duration;
    duration = asm_call_metrics.bn254_complex_mul_counter == 0 ? 0 : (asm_call_metrics.bn254_complex_mul_duration * 1000) / asm_call_metrics.bn254_complex_mul_counter;
    asm_call_total_duration += asm_call_metrics.bn254_complex_mul_duration;
    printf("bn254_complex_mul: counter = %lu, duration = %lu us, single duration = %lu ns, per thousand = %lu \n",
        asm_call_metrics.bn254_complex_mul_counter,
        asm_call_metrics.bn254_complex_mul_duration,
        duration,
        percentage);

    // Print total asm call percentage
    percentage = total_duration == 0 ? 0 : (asm_call_total_duration * 1000) / total_duration;
    printf("TOTAL: total duration = %lu us, asm call duration = %lu us, per thousand = %lu = %lu %%\n\n",
        total_duration,
        asm_call_total_duration,
        percentage,
        percentage/10);
}

#endif

uint64_t print_abcflag_counter = 0;

extern int _print_abcflag(uint64_t a, uint64_t b, uint64_t c, uint64_t flag)
{
    uint64_t * pMem = (uint64_t *)0xa0012118;
    printf("counter=%lu a=%08lx b=%08lx c=%08lx flag=%08lx mem=%08lx\n", print_abcflag_counter, a, b, c, flag, *pMem);
    // uint64_t *pRegs = (uint64_t *)RAM_ADDR;
    // for (int i=0; i<32; i++)
    // {
    //     printf("r%d=%08lx ", i, pRegs[i]);
    // }
    // printf("\n");
    fflush(stdout);
    print_abcflag_counter++;
    return 0;
}

uint64_t printed_chars_counter = 0;

extern int _print_char(uint64_t param)
{
    printed_chars_counter++;
    char c = param;
    printf("%c", c);
    return 0;
}

uint64_t print_step_counter = 0;
extern int _print_step(uint64_t step)
{
#ifdef DEBUG
    printf("step=%lu\n", print_step_counter);
    print_step_counter++;
    // struct timeval stop_time;
    // gettimeofday(&stop_time,NULL);
    // uint64_t duration = TimeDiff(start_time, stop_time);
    // uint64_t duration_s = duration/1000;
    // if (duration_s == 0) duration_s = 1;
    // uint64_t speed = step / duration_s;
    // if (emu_verbose) printf("print_step() Counter=%d Step=%d Duration=%dus Speed=%dsteps/ms\n", print_step_counter, step, duration, speed);
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
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) printf("opcode_keccak() calling KeccakF1600() counter=%lu address=%08lx\n", asm_call_metrics.keccak_counter, address);
#else
    if (emu_verbose) printf("opcode_keccak() calling KeccakF1600() address=%08lx\n", address);
#endif
#endif

    // Call keccak-f compression function
    keccakf1600_generic((uint64_t *)address);

#ifdef DEBUG
    if (emu_verbose) printf("opcode_keccak() called KeccakF1600()\n");
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.keccak_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.keccak_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_sha256(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) printf("opcode_sha256() calling sha256_transform_2() counter=%lu address=%p\n", asm_call_metrics.sha256_counter, address);
#else
    if (emu_verbose) printf("opcode_sha256() calling sha256_transform_2() address=%p\n", address);
#endif
#endif

    // Call SHA256 compression function
    zisk_sha256((uint64_t *)address[0], (uint64_t *)address[1]);   

#ifdef DEBUG
    if (emu_verbose) printf("opcode_sha256() called sha256_transform_2()\n");
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.sha256_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.sha256_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_arith256(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

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
        printf("opcode_arith256() calling Arith256() counter=%lu address=%p\n", asm_call_metrics.arith256_counter, address);
#else
        printf("opcode_arith256() calling Arith256() address=%p\n", address);
#endif
        printf("a = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", a[3], a[2], a[1], a[0], a[3], a[2], a[1], a[0]);
        printf("b = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", b[3], b[2], b[1], b[0], b[3], b[2], b[1], b[0]);
        printf("c = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", c[3], c[2], c[1], c[0], c[3], c[2], c[1], c[0]);
    }
#endif

    int result = Arith256 (a, b, c, dl, dh);
    if (result != 0)
    {
        printf("_opcode_arith256_add() failed callilng Arith256() result=%d;", result);
        exit(-1);
    }

#ifdef DEBUG
    if (emu_verbose) printf("opcode_arith256() called Arith256()\n");
    if (emu_verbose)
    {
        printf("dl = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", dl[3], dl[2], dl[1], dl[0], dl[3], dl[2], dl[1], dl[0]);
        printf("dh = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", dh[3], dh[2], dh[1], dh[0], dh[3], dh[2], dh[1], dh[0]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.arith256_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.arith256_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_arith256_mod(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

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
        printf("opcode_arith256_mod() calling Arith256Mod() counter=%lu address=%p\n", asm_call_metrics.arith256_mod_counter, address);
#else
        printf("opcode_arith256_mod() calling Arith256Mod() address=%p\n", address);
#endif
        printf("a = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", a[3], a[2], a[1], a[0], a[3], a[2], a[1], a[0]);
        printf("b = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", b[3], b[2], b[1], b[0], b[3], b[2], b[1], b[0]);
        printf("c = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", c[3], c[2], c[1], c[0], c[3], c[2], c[1], c[0]);
        printf("module = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", module[3], module[2], module[1], module[0], module[3], module[2], module[1], module[0]);
    }
#endif

    int result = Arith256Mod (a, b, c, module, d);
    if (result != 0)
    {
        printf("_opcode_arith256_mod() failed callilng Arith256Mod() result=%d;", result);
        exit(-1);
    }

#ifdef DEBUG
    if (emu_verbose) printf("opcode_arith256_mod() called Arith256Mod()\n");
    if (emu_verbose)
    {
        printf("d = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", d[3], d[2], d[1], d[0], d[3], d[2], d[1], d[0]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.arith256_mod_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.arith256_mod_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_secp256k1_add(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        printf("opcode_secp256k1_add() calling AddPointEcP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.secp256k1_add_counter, address, p1, p2);
#else
        printf("opcode_secp256k1_add() calling AddPointEcP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
        printf("p2.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0], p2[3], p2[2], p2[1], p2[0]);
        printf("p2.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4], p2[7], p2[6], p2[5], p2[4]);
    }
#endif
    int result = AddPointEcP (
        0,
        p1, // p1 = [x1, y1] = 8x64bits
        p2, // p2 = [x2, y2] = 8x64bits
        p1 // p3 = [x3, y3] = 8x64bits
    );
    if (result != 0)
    {
        printf("_opcode_secp256k1_add() failed callilng AddPointEcP() result=%d;", result);
        exit(-1);
    }
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("p3 = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.secp256k1_add_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.secp256k1_add_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_secp256k1_dbl(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

    uint64_t * p1 = address;

#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        printf("opcode_secp256k1_dbl() calling AddPointEcP() counter=%lu address=%p\n", asm_call_metrics.secp256k1_dbl_counter, address);
#else
        printf("opcode_secp256k1_dbl() calling AddPointEcP() address=%p\n", address);
#endif
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    int result = AddPointEcP (
        1,
        p1, // p1 = [x1, y1] = 8x64bits
        NULL, // p2 = [x2, y2] = 8x64bits
        p1 // p3 = [x3, y3] = 8x64bits
    );
    if (result != 0)
    {
        printf("_opcode_secp256k1_dbl() failed callilng AddPointEcP() result=%d;", result);
        exit(-1);
    }
#ifdef DEBUG
    if (emu_verbose) printf("opcode_secp256k1_dbl() called AddPointEcP()\n");
    if (emu_verbose)
    {
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.secp256k1_dbl_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.secp256k1_dbl_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern uint64_t MEM_TRACE_ADDRESS;
extern uint64_t fcall_ctx;
uint64_t print_fcall_ctx_counter = 0;

extern int _print_fcall_ctx(void)
{
    struct FcallContext * ctx = (struct FcallContext *)&fcall_ctx;
    printf("print_fcall_ctx(%lu) address=0x%p\n", print_fcall_ctx_counter, ctx);
    printf("\tfunction_id=0x%lu\n", ctx->function_id);
    printf("\tparams_max_size=%lu=0x%lx\n", ctx->params_max_size, ctx->params_max_size);
    printf("\tparams_size=0x%lu\n", ctx->params_size);
    for (int i=0; i<32; i++)
    {
        printf("\t\tparams[%d]=%lu=0x%lx\n", i, ctx->params[i], ctx->params[i]);
    }
    printf("\tresult_max_size=0x%lu\n", ctx->result_max_size);
    printf("\tresult_size=0x%lu\n", ctx->result_size);
    for (int i=0; i<32; i++)
    {
        printf("\t\tresult[%d]=%lu=0x%lx\n", i, ctx->result[i], ctx->result[i]);
    }
    printf("\n");
    print_fcall_ctx_counter++;
}

extern int _opcode_fcall(struct FcallContext * ctx)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) printf("_opcode_fcall() counter=%lu\n", asm_call_metrics.fcall_counter);
#else
    if (emu_verbose) printf("_opcode_fcall()\n");
#endif
#endif

    // Call fcall
    int iresult = Fcall(ctx);
    if (iresult < 0)
    {
        printf("_opcode_fcall() failed callilng Fcall() result=%d\n", iresult);
        exit(-1);
    }

#ifdef ASM_CALL_METRICS
    asm_call_metrics.fcall_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.fcall_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return iresult;
}

extern int _opcode_inverse_fp_ec(uint64_t params, uint64_t result)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) printf("_opcode_inverse_fp_ec() counter=%lu\n", asm_call_metrics.inverse_fp_ec_counter);
#else
    if (emu_verbose) printf("_opcode_inverse_fp_ec()\n");
#endif
#endif

    int iresult = InverseFpEc (
        (unsigned long *)params, // a
        (unsigned long *)result // r
    );
    if (iresult != 0)
    {
        printf("_opcode_inverse_fp_ec() failed callilng InverseFpEc() result=%d;", iresult);
        exit(-1);
    }

#ifdef ASM_CALL_METRICS
    asm_call_metrics.inverse_fp_ec_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.inverse_fp_ec_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_inverse_fn_ec(uint64_t params, uint64_t result)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) printf("_opcode_inverse_fn_ec() counter=%lu\n", asm_call_metrics.inverse_fn_ec_counter);
#else
    if (emu_verbose) printf("_opcode_inverse_fn_ec()\n");
#endif
#endif
    int iresult = InverseFnEc (
        (unsigned long *)params, // a
        (unsigned long *)result // r
    );
    if (iresult != 0)
    {
        printf("_opcode_inverse_fn_ec() failed callilng InverseFnEc() result=%d;", iresult);
        exit(-1);
    }

#ifdef ASM_CALL_METRICS
    asm_call_metrics.inverse_fn_ec_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.inverse_fn_ec_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_sqrt_fp_ec_parity(uint64_t params, uint64_t result)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif
#ifdef DEBUG
#ifdef ASM_CALL_METRICS
    if (emu_verbose) printf("_opcode_sqrt_fp_ec_parity() counter=%lu\n", asm_call_metrics.fcall_counter);
#else
    if (emu_verbose) printf("_opcode_sqrt_fp_ec_parity()\n");
#endif
#endif
    int iresult = SqrtFpEcParity (
        (unsigned long *)params, // a
        *(unsigned long *)(params + 4*8), // parity
        (unsigned long *)result // r
    );
    if (iresult != 0)
    {
        printf("_opcode_sqrt_fp_ec_parity() failed callilng SqrtFpEcParity() result=%d;", iresult);
        exit(-1);
    }

#ifdef ASM_CALL_METRICS
    asm_call_metrics.sqrt_fp_ec_parity_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.sqrt_fp_ec_parity_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

/*********/
/* BN254 */
/*********/

extern int _opcode_bn254_curve_add(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        printf("_opcode_bn254_curve_add() calling AddPointEcP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.bn254_curve_add_counter, address, p1, p2);
#else
        printf("_opcode_bn254_curve_add() calling AddPointEcP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
        printf("p2.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0], p2[3], p2[2], p2[1], p2[0]);
        printf("p2.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4], p2[7], p2[6], p2[5], p2[4]);
    }
#endif
    int result = BN254CurveAddP (
        p1, // p1 = [x1, y1] = 8x64bits
        p2, // p2 = [x2, y2] = 8x64bits
        p1 // p3 = [x3, y3] = 8x64bits
    );
    if (result != 0)
    {
        printf("_opcode_bn254_curve_add() failed callilng BN254CurveAddP() result=%d;", result);
        exit(-1);
    }
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.bn254_curve_add_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.bn254_curve_add_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_bn254_curve_dbl(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

    uint64_t * p1 = address;
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        printf("_opcode_bn254_curve_dbl() calling BN254CurveDblP() counter=%lu address=%p p1_address=%p\n", asm_call_metrics.bn254_curve_dbl_counter, address, p1);
#else
        printf("_opcode_bn254_curve_dbl() calling BN254CurveDblP() address=%p p1_address=%p\n", address, p1);
#endif
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    int result = BN254CurveDblP (
        p1, // p1 = [x1, y1] = 8x64bits
        p1 // p1 = [x1, y1] = 8x64bits
    );
    if (result != 0)
    {
        printf("_opcode_bn254_curve_dbl() failed callilng BN254CurveDblP() result=%d;", result);
        exit(-1);
    }
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.bn254_curve_dbl_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.bn254_curve_dbl_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_bn254_complex_add(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        printf("_opcode_bn254_complex_add() calling BN254ComplexAddP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.bn254_complex_add_counter, address, p1, p2);
#else
        printf("_opcode_bn254_complex_add() calling BN254ComplexAddP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
        printf("p2.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0], p2[3], p2[2], p2[1], p2[0]);
        printf("p2.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4], p2[7], p2[6], p2[5], p2[4]);
    }
#endif
    int result = BN254ComplexAddP (
        p1, // p1 = [x1, y1] = 8x64bits
        p2, // p2 = [x2, y2] = 8x64bits
        p1 // p3 = [x3, y3] = 8x64bits
    );
    if (result != 0)
    {
        printf("_opcode_bn254_complex_add() failed callilng BN254ComplexAddP() result=%d;", result);
        exit(-1);
    }
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.bn254_complex_add_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.bn254_complex_add_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_bn254_complex_sub(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        printf("_opcode_bn254_complex_sub() calling BN254ComplexSubP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.bn254_complex_sub_counter, address, p1, p2);
#else
        printf("_opcode_bn254_complex_sub() calling BN254ComplexSubP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
        printf("p2.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0], p2[3], p2[2], p2[1], p2[0]);
        printf("p2.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4], p2[7], p2[6], p2[5], p2[4]);
    }
#endif
    int result = BN254ComplexSubP (
        p1, // p1 = [x1, y1] = 8x64bits
        p2, // p2 = [x2, y2] = 8x64bits
        p1 // p3 = [x3, y3] = 8x64bits
    );
    if (result != 0)
    {
        printf("_opcode_bn254_complex_sub() failed callilng BN254ComplexSubP() result=%d;", result);
        exit(-1);
    }
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.bn254_complex_sub_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.bn254_complex_sub_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}

extern int _opcode_bn254_complex_mul(uint64_t * address)
{
#ifdef ASM_CALL_METRICS
    gettimeofday(&asm_call_start, NULL);
#endif

    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
#ifdef ASM_CALL_METRICS
        printf("_opcode_bn254_complex_mul() calling BN254ComplexMulP() counter=%lu address=%p p1_address=%p p2_address=%p\n", asm_call_metrics.bn254_complex_mul_counter, address, p1, p2);
#else
        printf("_opcode_bn254_complex_mul() calling BN254ComplexMulP() address=%p p1_address=%p p2_address=%p\n", address, p1, p2);
#endif
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
        printf("p2.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[3], p2[2], p2[1], p2[0], p2[3], p2[2], p2[1], p2[0]);
        printf("p2.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p2[7], p2[6], p2[5], p2[4], p2[7], p2[6], p2[5], p2[4]);
    }
#endif
    int result = BN254ComplexMulP (
        p1, // p1 = [x1, y1] = 8x64bits
        p2, // p2 = [x2, y2] = 8x64bits
        p1 // p3 = [x3, y3] = 8x64bits
    );
    if (result != 0)
    {
        printf("_opcode_bn254_complex_mul() failed callilng BN254ComplexMulP() result=%d;", result);
        exit(-1);
    }
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
#ifdef ASM_CALL_METRICS
    asm_call_metrics.bn254_complex_mul_counter++;
    gettimeofday(&asm_call_stop, NULL);
    asm_call_metrics.bn254_complex_mul_duration += TimeDiff(asm_call_start, asm_call_stop);
#endif
    return 0;
}