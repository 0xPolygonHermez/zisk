#define _GNU_SOURCE
#include <stdio.h>
#include <sys/mman.h>
#include <errno.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <assert.h>
#include <sys/time.h>
#include <semaphore.h>
#include <fcntl.h>
#include <sys/file.h>
#include "c_provided.hpp"
#include "globals.hpp"
#include "trace.hpp"
#include "emu.hpp"

/**************/
/* TRACE SIZE */
/**************/

void set_trace_size (uint64_t new_trace_size)
{
    // Update trace global variables
    // printf("%s trace resize (trace_resize_request: %ld):  %ld MB => %ld MB\n", log_name, trace_resize_request, trace_size >> 20, new_trace_size >> 20);
    
    // trace_resize_request = 0;

    trace_size = new_trace_size;
    trace_address_threshold = TRACE_ADDR + trace_size - MAX_CHUNK_TRACE_SIZE;
    pOutputTrace[2] = trace_size;    
}

/**************/
/* PRINT REGS */
/**************/

//#define PRINT_REGS
#ifdef PRINT_REGS
extern uint64_t reg_0;
extern uint64_t reg_1;
extern uint64_t reg_2;
extern uint64_t reg_3;
extern uint64_t reg_4;
extern uint64_t reg_5;
extern uint64_t reg_6;
extern uint64_t reg_7;
extern uint64_t reg_8;
extern uint64_t reg_9;
extern uint64_t reg_10;
extern uint64_t reg_11;
extern uint64_t reg_12;
extern uint64_t reg_13;
extern uint64_t reg_14;
extern uint64_t reg_15;
extern uint64_t reg_16;
extern uint64_t reg_17;
extern uint64_t reg_18;
extern uint64_t reg_19;
extern uint64_t reg_20;
extern uint64_t reg_21;
extern uint64_t reg_22;
extern uint64_t reg_23;
extern uint64_t reg_24;
extern uint64_t reg_25;
extern uint64_t reg_26;
extern uint64_t reg_27;
extern uint64_t reg_28;
extern uint64_t reg_29;
extern uint64_t reg_30;
extern uint64_t reg_31;
extern uint64_t reg_32;
extern uint64_t reg_33;
extern uint64_t reg_34;
#endif

// Used for debugging purposes
extern int _print_regs()
{
#ifdef PRINT_REGS
    printf("print_regs()\n");
    printf("\treg[ 0]=%lu=0x%lx=@%p\n", reg_0,  reg_0,  &reg_0);
    printf("\treg[ 1]=%lu=0x%lx=@%p\n", reg_1,  reg_1,  &reg_1);
    printf("\treg[ 2]=%lu=0x%lx=@%p\n", reg_2,  reg_2,  &reg_2);
    printf("\treg[ 3]=%lu=0x%lx=@%p\n", reg_3,  reg_3,  &reg_3);
    printf("\treg[ 4]=%lu=0x%lx=@%p\n", reg_4,  reg_4,  &reg_4);
    printf("\treg[ 5]=%lu=0x%lx=@%p\n", reg_5,  reg_5,  &reg_5);
    printf("\treg[ 6]=%lu=0x%lx=@%p\n", reg_6,  reg_6,  &reg_6);
    printf("\treg[ 7]=%lu=0x%lx=@%p\n", reg_7,  reg_7,  &reg_7);
    printf("\treg[ 8]=%lu=0x%lx=@%p\n", reg_8,  reg_8,  &reg_8);
    printf("\treg[ 9]=%lu=0x%lx=@%p\n", reg_9,  reg_9,  &reg_9);
    printf("\treg[10]=%lu=0x%lx=@%p\n", reg_10, reg_10, &reg_10);
    printf("\treg[11]=%lu=0x%lx=@%p\n", reg_11, reg_11, &reg_11);
    printf("\treg[12]=%lu=0x%lx=@%p\n", reg_12, reg_12, &reg_12);
    printf("\treg[13]=%lu=0x%lx=@%p\n", reg_13, reg_13, &reg_13);
    printf("\treg[14]=%lu=0x%lx=@%p\n", reg_14, reg_14, &reg_14);
    printf("\treg[15]=%lu=0x%lx=@%p\n", reg_15, reg_15, &reg_15);
    printf("\treg[16]=%lu=0x%lx=@%p\n", reg_16, reg_16, &reg_16);
    printf("\treg[17]=%lu=0x%lx=@%p\n", reg_17, reg_17, &reg_17);
    printf("\treg[18]=%lu=0x%lx=@%p\n", reg_18, reg_18, &reg_18);
    printf("\treg[19]=%lu=0x%lx=@%p\n", reg_19, reg_19, &reg_19);
    printf("\treg[20]=%lu=0x%lx=@%p\n", reg_20, reg_20, &reg_20);
    printf("\treg[21]=%lu=0x%lx=@%p\n", reg_21, reg_21, &reg_21);
    printf("\treg[22]=%lu=0x%lx=@%p\n", reg_22, reg_22, &reg_22);
    printf("\treg[23]=%lu=0x%lx=@%p\n", reg_23, reg_23, &reg_23);
    printf("\treg[24]=%lu=0x%lx=@%p\n", reg_24, reg_24, &reg_24);
    printf("\treg[25]=%lu=0x%lx=@%p\n", reg_25, reg_25, &reg_25);
    printf("\treg[26]=%lu=0x%lx=@%p\n", reg_26, reg_26, &reg_26);
    printf("\treg[27]=%lu=0x%lx=@%p\n", reg_27, reg_27, &reg_27);
    printf("\treg[28]=%lu=0x%lx=@%p\n", reg_28, reg_28, &reg_28);
    printf("\treg[29]=%lu=0x%lx=@%p\n", reg_29, reg_29, &reg_29);
    printf("\treg[30]=%lu=0x%lx=@%p\n", reg_30, reg_30, &reg_30);
    printf("\treg[31]=%lu=0x%lx=@%p\n", reg_31, reg_31, &reg_31);
    printf("\treg[32]=%lu=0x%lx=@%p\n", reg_32, reg_32, &reg_32);
    printf("\treg[33]=%lu=0x%lx=@%p\n", reg_33, reg_33, &reg_33);
    printf("\treg[34]=%lu=0x%lx=@%p\n", reg_34, reg_34, &reg_34);
    printf("\n");
#endif
    return 0;
}

/************/
/* PRINT PC */
/************/

//#define PRINT_PC_DURATION
#ifdef PRINT_PC_DURATION
struct timeval print_pc_tv;
#endif

// Used for debugging purposes
extern int _print_pc (uint64_t pc, uint64_t c)
{
#ifdef PRINT_PC_DURATION
    print_pc_counter++;
    {
        struct timeval tv;
        gettimeofday(&tv, NULL);
        uint64_t duration = TimeDiff(print_pc_tv, tv);
        if (duration > 900)
        {
            uint64_t chunk = print_pc_counter / chunk_size;
            printf("print_pc() pc=%lx counter=%lu sec=%lu usec=%lu duration=%lu chunk=%lu\n", pc, print_pc_counter, tv.tv_sec, tv.tv_usec, duration, chunk);
            fflush(stdout);
        }
        print_pc_tv = tv;
    }
#endif

    printf("s=%lu pc=%lx c=%lx", print_pc_counter, pc, c);

//#define PRINT_PC_REGS
#ifdef PRINT_PC_REGS
    /* Used for debugging */
    printf(" r0=%lx", reg_0);
    printf(" r1=%lx", reg_1);
    printf(" r2=%lx", reg_2);
    printf(" r3=%lx", reg_3);
    printf(" r4=%lx", reg_4);
    printf(" r5=%lx", reg_5);
    printf(" r6=%lx", reg_6);
    printf(" r7=%lx", reg_7);
    printf(" r8=%lx", reg_8);
    printf(" r9=%lx", reg_9);
    printf(" r10=%lx", reg_10);
    printf(" r11=%lx", reg_11);
    printf(" r12=%lx", reg_12);
    printf(" r13=%lx", reg_13);
    printf(" r14=%lx", reg_14);
    printf(" r15=%lx", reg_15);
    printf(" r16=%lx", reg_16);
    printf(" r17=%lx", reg_17);
    printf(" r18=%lx", reg_18);
    printf(" r19=%lx", reg_19);
    printf(" r20=%lx", reg_20);
    printf(" r21=%lx", reg_21);
    printf(" r22=%lx", reg_22);
    printf(" r23=%lx", reg_23);
    printf(" r24=%lx", reg_24);
    printf(" r25=%lx", reg_25);
    printf(" r26=%lx", reg_26);
    printf(" r27=%lx", reg_27);
    printf(" r28=%lx", reg_28);
    printf(" r29=%lx", reg_29);
    printf(" r30=%lx", reg_30);
    printf(" r31=%lx", reg_31);
#endif

    printf("\n");
    fflush(stdout);
    print_pc_counter++;
    return 0;
}

/**************/
/* CHUNK DONE */
/**************/

//#define CHUNK_DONE_DURATION
#ifdef CHUNK_DONE_DURATION
uint64_t chunk_done_counter = 0;
struct timeval chunk_done_tv;
#endif

//#define CHUNK_DONE_SYNC_DURATION
#ifdef CHUNK_DONE_SYNC_DURATION
struct timeval sync_start, sync_stop;
uint64_t sync_duration = 0;
#endif

// Called by the assembly to notify that a chunk is done and its trace is ready to be consumed
extern void _chunk_done()
{
#ifdef CHUNK_DONE_DURATION
    chunk_done_counter++;
    if ((chunk_done_counter & 0xFF) == 0)
    {
        struct timeval tv;
        gettimeofday(&tv, NULL);
        uint64_t duration = TimeDiff(chunk_done_tv, tv);
        if (duration > 5000)
        {
            printf("chunk_done() counter=%lu sec=%lu usec=%lu duration=%lu\n", chunk_done_counter, tv.tv_sec, tv.tv_usec, duration);
            fflush(stdout);
        }
        chunk_done_tv = tv;
    }
#endif

#ifdef CHUNK_DONE_SYNC_DURATION
    gettimeofday(&sync_start, NULL);
#endif

    __sync_synchronize();

#ifdef CHUNK_DONE_SYNC_DURATION
    gettimeofday(&sync_stop, NULL);
    sync_duration += TimeDiff(sync_start, sync_stop);
    printf("chunk_done() sync_duration=%lu\n", sync_duration);
#endif

    // Notify the caller that a new chunk is done and its trace is ready to be consumed
    assert(call_chunk_done);
    int result = sem_post(sem_chunk_done);
    if (result == -1)
    {
        printf("ERROR: Failed calling sem_post(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
}

/*****************/
/* REALLOC TRACE */
/*****************/

// Called by the assembly to reallocate the trace when needed, e.g. for the next chunk,
// to increase the trace size by another chunk size
extern void _realloc_trace (void)
{
    // Increase realloc counter
    realloc_counter++;

    // Map next chunk of the trace shared memory
    trace_map_next_chunk();

    // Update trace global variables
    set_trace_size(trace_total_mapped_size);

#ifdef DEBUG
    if (verbose) printf("realloc_trace() realloc counter=%lu trace_address=0x%lx trace_size=%lu=%lx max_address=0x%lx trace_address_threshold=0x%lx chunk_size=%lu\n", realloc_counter, trace_address, trace_size, trace_size, trace_address + trace_size, trace_address_threshold, chunk_size);
#endif
}

/*********************************/
/* WAIT FOR PRECOMPILE AVAILABLE */
/*********************************/

// Called by the assembly when prec_written == prec_read, to wait for new precompile results to be available
int _wait_for_prec_avail (void)
{
    // Increment wait counter
    wait_prec_avail_counter++;

    //printf("wait_for_prec_avail() counter=%lu\n", wait_prec_avail_counter);

    // Sync control output shared memory so that the writer can see the precompile reads we have
    // done, and thus update the precompile_written_address if needed
    if (msync((void *)shmem_control_output_address, CONTROL_OUTPUT_SIZE, MS_SYNC) != 0) {
        printf("ERROR: msync failed for shmem_control_output_address errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Tell the writer that we have read some precompile results
    sem_post(sem_prec_read);

    // Make sure the precompile available semaphore is reset before checking the condition,
    // since the caller may have posted it (even several times) before we called sem_wait()
    while (sem_trywait(sem_prec_avail) == 0) {/*printf("Purging sem_prec_avail\n");*/};

    // Sync control input shared memory so that we can see the latest precompile_written_address value
    if (msync((void *)shmem_control_input_address, CONTROL_INPUT_SIZE, MS_SYNC) != 0) {
        printf("ERROR: msync failed for shmem_control_input_address errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Check if there are already precompile results available
    if (*precompile_written_address > *precompile_read_address)
    {
        // Sync precompile shared memory
        if (msync((void *)shmem_precompile_address, MAX_PRECOMPILE_SIZE, MS_SYNC) != 0) {
            printf("ERROR: msync failed for shmem_precompile_address errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        return 0;
    }

    // Wait again, but blocking this time
    while (true)
    {
        struct timespec ts;
        int result = clock_gettime(CLOCK_REALTIME, &ts);
        if (result == -1)
        {
            printf("ERROR: wait_for_prec_avail() failed calling clock_gettime() errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        ts.tv_sec += 5; // 5 seconds timeout

        //printf("_wait_for_prec_avail() calling sem_wait precompile_written_address=%lu precompile_read_address=%lu\n", *precompile_written_address, *precompile_read_address);
        if (wait_flag) *waiting_for_precompile_address = wait_prec_avail_counter << 1; // Leave a mark in shmem that we are waiting; for debugging purposes
        result = sem_timedwait(sem_prec_avail, &ts);
        if (wait_flag) *waiting_for_precompile_address = (wait_prec_avail_counter << 1) + 1; // Clear the mark in shmem that we are waiting; for debugging purposes
        //printf("_wait_for_prec_avail() called sem_wait precompile_written_address=%lu precompile_read_address=%lu\n", *precompile_written_address, *precompile_read_address);
        if ((result == -1) && (errno != ETIMEDOUT))
        {
            printf("ERROR: wait_for_prec_avail() failed calling sem_wait(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Sync control input shared memory so that we can see the latest precompile_written_address value
        if (msync((void *)shmem_control_input_address, CONTROL_INPUT_SIZE, MS_SYNC) != 0) {
            printf("ERROR: msync failed for shmem_control_input_address errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        if (*precompile_exit_address != 0)
        {
            printf("ERROR: wait_for_prec_avail() found precompile_exit_address=%lu\n", *precompile_exit_address);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (*precompile_written_address > *precompile_read_address)
        {
            // Sync precompile shared memory
            if (msync((void *)shmem_precompile_address, MAX_PRECOMPILE_SIZE, MS_SYNC) != 0) {
                printf("ERROR: msync failed for shmem_precompile_address errno=%d=%s\n", errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            return 0;
        }
    }

    printf("ERROR: wait_for_prec_avail() unreachable code\n");
    fflush(stdout);
    fflush(stderr);
    exit(-1);
}

/****************************/
/* WAIT FOR INPUT AVAILABLE */
/****************************/

// Called by the assembly when input_written == input_read, to wait for new input to be available
int _wait_for_input_avail (uint64_t required_input_bytes)
{
    // Increment wait counter
    wait_input_avail_counter++;

    //printf("wait_for_input_avail() required_input_bytes=%lu counter=%lu\n", required_input_bytes, wait_input_avail_counter);

    // Make sure the input available semaphore is reset before checking the condition,
    // since the caller may have posted it (even several times) before we called sem_wait()
    while (sem_trywait(sem_input_avail) == 0) {/*printf("Purging sem_input_avail\n");*/};
    
    // Sync control input shared memory so that we can see the latest input_written_address value
    if (msync((void *)shmem_control_input_address, CONTROL_INPUT_SIZE, MS_SYNC) != 0) {
        printf("ERROR: msync failed for shmem_control_input_address errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Check if there is already input data available
    if (*input_written_address > required_input_bytes)
    {
        // Sync input shared memory
        if (msync((void *)INPUT_ADDR, MAX_INPUT_SIZE, MS_SYNC) != 0) 
        {
            printf("ERROR: msync failed for shmem_input_address errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        return 0;
    }

    // Wait again, but blocking this time
    while (true)
    {
        struct timespec ts;
        int result = clock_gettime(CLOCK_REALTIME, &ts);
        if (result == -1)
        {
            printf("ERROR: wait_for_input_avail() failed calling clock_gettime() errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        ts.tv_sec += 5; // 5 seconds timeout

        //printf("_wait_for_input_avail() calling sem_wait input_written_address=%lu required_input_bytes=%lu\n", *input_written_address, required_input_bytes);
        if (wait_flag) *waiting_for_input_address = wait_input_avail_counter << 1; // Leave a mark in shmem that we are waiting; for debugging purposes
        result = sem_timedwait(sem_input_avail, &ts);
        if (wait_flag) *waiting_for_input_address = (wait_input_avail_counter << 1) + 1; // Clear the mark in shmem that we are waiting; for debugging purposes
        //printf("_wait_for_input_avail() called sem_wait input_written_address=%lu required_input_bytes=%lu\n", *input_written_address, required_input_bytes);
        if ((result == -1) && (errno != ETIMEDOUT))
        {
            printf("ERROR: wait_for_input_avail() failed calling sem_wait(%s) errno=%d=%s\n", sem_input_avail_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Sync control input shared memory so that we can see the latest input_written_address value
        if (msync((void *)shmem_control_input_address, CONTROL_INPUT_SIZE, MS_SYNC) != 0) {
            printf("ERROR: msync failed for shmem_control_input_address errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        if (*precompile_exit_address != 0)
        {
            printf("ERROR: wait_for_input_avail() found precompile_exit_address=%lu\n", *precompile_exit_address);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (*input_written_address >= required_input_bytes)
        {
            // Sync input shared memory
            if (msync((void *)INPUT_ADDR, MAX_INPUT_SIZE, MS_SYNC) != 0) {
                printf("ERROR: msync failed for shmem_input_address errno=%d=%s\n", errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            return 0;
        }
    }

    printf("ERROR: wait_for_input_avail() unreachable code\n");
    fflush(stdout);
    fflush(stderr);
    exit(-1);
}