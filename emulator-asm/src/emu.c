
#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>
#include <sys/time.h>
#include <errno.h>
#include <unistd.h>
#include <stdlib.h>
#include "../../lib-c/c/src/ec/ec.hpp"
#include "../../lib-c/c/src/fcall/fcall.hpp"
#include "../../lib-c/c/src/arith256/arith256.hpp"
#include "bcon/bcon_sha256.hpp"

extern void keccakf1600_generic(uint64_t state[25]);

#ifdef DEBUG
bool emu_verbose = false;
bool keccak_metrics = false;
bool sha256_metrics = false;
bool arith256_metrics = false;
bool arith256_mod_metrics = false;
bool secp256k1_add_metrics = false;
bool secp256k1_dbl_metrics = false;
#endif

struct timeval keccak_start, keccak_stop;
uint64_t keccak_counter = 0;
uint64_t keccak_duration = 0;

struct timeval sha256_start, sha256_stop;
uint64_t sha256_counter = 0;
uint64_t sha256_duration = 0;

struct timeval arith256_start, arith256_stop;
uint64_t arith256_counter = 0;
uint64_t arith256_duration = 0;

struct timeval arith256_mod_start, arith256_mod_stop;
uint64_t arith256_mod_counter = 0;
uint64_t arith256_mod_duration = 0;

struct timeval secp256k1_add_start, secp256k1_add_stop;
uint64_t secp256k1_add_counter = 0;
uint64_t secp256k1_add_duration = 0;

struct timeval secp256k1_dbl_start, secp256k1_dbl_stop;
uint64_t secp256k1_dbl_counter = 0;
uint64_t secp256k1_dbl_duration = 0;

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
#ifdef DEBUG
    if (keccak_metrics || emu_verbose) gettimeofday(&keccak_start, NULL);
    if (emu_verbose) printf("opcode_keccak() calling KeccakF1600() counter=%lu address=%08lx\n", keccak_counter, address);
#endif
    keccakf1600_generic((uint64_t *)address);
#ifdef DEBUG
    if (emu_verbose) printf("opcode_keccak() called KeccakF1600()\n");
    keccak_counter++;
    if (keccak_metrics || emu_verbose)
    {
        gettimeofday(&keccak_stop, NULL);
        keccak_duration += TimeDiff(keccak_start, keccak_stop);
    }
#endif
    return 0;
}

extern int _opcode_sha256(uint64_t * address)
{
#ifdef DEBUG
    if (sha256_metrics || emu_verbose) gettimeofday(&sha256_start, NULL);
    if (emu_verbose) printf("opcode_sha256() calling sha256_transform_2() counter=%lu address=%p\n", sha256_counter, address);
#endif

    sha256_transform_2( (uint32_t *) address, (uint8_t *)(address + 4));

#ifdef DEBUG
    if (emu_verbose) printf("opcode_sha256() called sha256_transform_2()\n");
    sha256_counter++;
    if (sha256_metrics || emu_verbose)
    {
        gettimeofday(&sha256_stop, NULL);
        sha256_duration += TimeDiff(sha256_start, sha256_stop);
    }
#endif
    return 0;
}

extern int _opcode_arith256(uint64_t * address)
{
#ifdef DEBUG
    if (arith256_metrics || emu_verbose) gettimeofday(&arith256_start, NULL);
#endif
    uint64_t * a = (uint64_t *)address[0];
    uint64_t * b = (uint64_t *)address[1];
    uint64_t * c = (uint64_t *)address[2];
    uint64_t * dl = (uint64_t *)address[3];
    uint64_t * dh = (uint64_t *)address[4];
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("opcode_arith256() calling Arith256() counter=%lu address=%p\n", arith256_counter, address);
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
    arith256_counter++;
    if (arith256_metrics || emu_verbose)
    {
        gettimeofday(&arith256_stop, NULL);
        arith256_duration += TimeDiff(arith256_start, arith256_stop);
    }
#endif
    return 0;
}

extern int _opcode_arith256_mod(uint64_t * address)
{
#ifdef DEBUG
    if (arith256_mod_metrics || emu_verbose) gettimeofday(&arith256_mod_start, NULL);
#endif
    uint64_t * a = (uint64_t *)address[0];
    uint64_t * b = (uint64_t *)address[1];
    uint64_t * c = (uint64_t *)address[2];
    uint64_t * module = (uint64_t *)address[3];
    uint64_t * d = (uint64_t *)address[4];
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("opcode_arith256_mod() calling Arith256Mod() counter=%lu address=%p\n", arith256_mod_counter, address);
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
    arith256_mod_counter++;
    if (arith256_mod_metrics || emu_verbose)
    {
        gettimeofday(&arith256_mod_stop, NULL);
        arith256_mod_duration += TimeDiff(arith256_mod_start, arith256_mod_stop);
    }
#endif
    return 0;
}

extern int _opcode_secp256k1_add(uint64_t * address)
{
#ifdef DEBUG
    if (secp256k1_add_metrics || emu_verbose) gettimeofday(&secp256k1_add_start, NULL);
#endif
    uint64_t * p1 = (uint64_t *)address[0];
    uint64_t * p2 = (uint64_t *)address[1];
#ifdef DEBUG
    if (emu_verbose)
    {
        printf("opcode_secp256k1_add() calling AddPointEcP() counter=%lu address=%p p1_address=%p p2_address=%p\n", secp256k1_add_counter, address, p1, p2);
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
    secp256k1_add_counter++;
    if (secp256k1_add_metrics || emu_verbose)
    {
        gettimeofday(&secp256k1_add_stop, NULL);
        secp256k1_add_duration += TimeDiff(secp256k1_add_start, secp256k1_add_stop);
    }
#endif
    return 0;
}

extern int _opcode_secp256k1_dbl(uint64_t * address)
{
#ifdef DEBUG
    if (secp256k1_dbl_metrics || emu_verbose) gettimeofday(&secp256k1_dbl_start, NULL);
#endif

    uint64_t * p1 = address;

#ifdef DEBUG
    if (emu_verbose)
    {
        printf("opcode_secp256k1_dbl() calling AddPointEcP() counter=%lu address=%p\n", secp256k1_dbl_counter, address);
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
    secp256k1_dbl_counter++;
    if (secp256k1_dbl_metrics || emu_verbose)
    {
        gettimeofday(&secp256k1_dbl_stop, NULL);
        secp256k1_dbl_duration += TimeDiff(secp256k1_dbl_start, secp256k1_dbl_stop);
    }
    if (emu_verbose)
    {
        printf("p1.x = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[3], p1[2], p1[1], p1[0], p1[3], p1[2], p1[1], p1[0]);
        printf("p1.y = %lu:%lu:%lu:%lu = %lx:%lx:%lx:%lx\n", p1[7], p1[6], p1[5], p1[4], p1[7], p1[6], p1[5], p1[4]);
    }
#endif
    return 0;
}

uint64_t fcall_counter = 0;
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
#ifdef DEBUG
        if (emu_verbose) printf("_opcode_fcall() counter=%lu\n", fcall_counter);
#endif
    fcall_counter++;
    //printf("_opcode_fcall() counter=%lu\n", fcall_counter);
    int iresult = Fcall(ctx);
    if (iresult < 0)
    {
        printf("_opcode_fcall() failed callilng Fcall() result=%d\n", iresult);
        exit(-1);
    }
    return iresult;
}

extern int _opcode_inverse_fp_ec(uint64_t params, uint64_t result)
{
#ifdef DEBUG
    if (emu_verbose) printf("_opcode_inverse_fp_ec() counter=%lu\n", fcall_counter);
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
    return 0;
}

extern int _opcode_inverse_fn_ec(uint64_t params, uint64_t result)
{
#ifdef DEBUG
    if (emu_verbose) printf("_opcode_inverse_fn_ec() counter=%lu\n", fcall_counter);
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
    return 0;
}

extern int _opcode_sqrt_fp_ec_parity(uint64_t params, uint64_t result)
{
#ifdef DEBUG
    if (emu_verbose) printf("_opcode_sqrt_fp_ec_parity() counter=%lu\n", fcall_counter);
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
    return 0;
}