#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <errno.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <assert.h>
#include <sys/mman.h>
#include <sys/time.h>
#include <semaphore.h>
#include <fcntl.h>
#include <sys/file.h>
#include <unistd.h>
#include "server.hpp"
#include "globals.hpp"
#include "asm_provided.hpp"
#include "trace_logs.hpp"
#include "trace.hpp"
#include "emu.hpp"
#include "c_provided.hpp"

/**********/
/* SERVER */
/**********/

// ROM histogram
uint64_t histogram_size = 0;
uint64_t bios_size = 0;
uint64_t program_size = 0;

void server_setup (void)
{
    assert(server);
    assert(!client);

    int result;

    /*******/
    /* ROM */
    /*******/
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {

        if (verbose) gettimeofday(&start_time, NULL);
        void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED | map_locked_flag, -1, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pRom == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(rom) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pRom != ROM_ADDR)
        {
            printf("ERROR: Called mmap(rom) but returned address = %p != 0x%lx\n", pRom, ROM_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(rom) mapped %ld B and returned address %p in %lu us\n", ROM_SIZE, pRom, duration);
    }

    /*********/
    /* INPUT */
    /*********/

    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
        if (!open_input_shm)
        {
            // Make sure the input shared memory is deleted
            shm_unlink(shmem_input_name);

            // Create the input shared memory
            shmem_input_fd = shm_open(shmem_input_name, O_RDWR | O_CREAT | O_EXCL, 0666);
            if (shmem_input_fd < 0)
            {
                printf("ERROR: Failed calling input RW shm_open(%s) as read-write errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            // Size it
            result = ftruncate(shmem_input_fd, MAX_INPUT_SIZE);
            if (result != 0)
            {
                printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            // Sync
            fsync(shmem_input_fd);

            // Close the descriptor
            if (close(shmem_input_fd) != 0)
            {
                printf("ERROR: Failed calling close(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }
        }

        // Open the input shared memory as read-only
        shmem_input_fd = shm_open(shmem_input_name, O_RDONLY, 0666);
        if (shmem_input_fd < 0)
        {
            printf("ERROR: Failed calling input RO shm_open(%s) as read-only errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map input address space
        if (verbose) gettimeofday(&start_time, NULL);
        void * pInput = mmap((void *)INPUT_ADDR, MAX_INPUT_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_input_fd, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pInput == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(input) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pInput != INPUT_ADDR)
        {
            printf("ERROR: Called mmap(pInput) but returned address = %p != 0x%lx\n", pInput, INPUT_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) {
            printf("mmap(input) mapped %lu B and returned address %p in %lu us\n", MAX_INPUT_SIZE, pInput, duration);
            fflush(stdout);
        }
    }

    /**********************/
    /* PRECOMPILE_RESULTS */
    /**********************/

    if (precompile_results_enabled)
    {
        /**************/
        /* PRECOMPILE */
        /**************/

        if (!open_input_shm)
        {
            // Make sure the precompile results shared memory is deleted
            shm_unlink(shmem_precompile_name);

            // Create the precompile results shared memory
            shmem_precompile_fd = shm_open(shmem_precompile_name, O_RDWR | O_CREAT, 0666);
            if (shmem_precompile_fd < 0)
            {
                printf("ERROR: Failed calling precompile shm_open(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            // Size it
            result = ftruncate(shmem_precompile_fd, MAX_PRECOMPILE_SIZE);
            if (result != 0)
            {
                printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            // Sync
            fsync(shmem_precompile_fd);

            // Close the descriptor
            if (close(shmem_precompile_fd) != 0)
            {
                printf("ERROR: Failed calling close(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }
        }

        // Open the precompile shared memory as read-only
        shmem_precompile_fd = shm_open(shmem_precompile_name, O_RDONLY, 0666);
        if (shmem_precompile_fd < 0)
        {
            printf("ERROR: Failed calling precompile RO shm_open(%s) as read-only errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map precompile address space
        if (verbose) gettimeofday(&start_time, NULL);
        void * pPrecompile = mmap(NULL, MAX_PRECOMPILE_SIZE, PROT_READ, MAP_SHARED | map_locked_flag, shmem_precompile_fd, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pPrecompile == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(precompile) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        shmem_precompile_address = pPrecompile;
        precompile_results_address = (uint64_t *)pPrecompile;
        if (verbose) printf("mmap(precompile) mapped %lu B and returned address %p in %lu us\n", MAX_PRECOMPILE_SIZE, precompile_results_address, duration);

        /*****************/
        /* CONTROL INPUT */
        /*****************/

        if (!open_input_shm)
        {
            // Make sure the precompile results shared memory is deleted
            shm_unlink(shmem_control_input_name);

            // Create the control shared memory
            shmem_control_input_fd = shm_open(shmem_control_input_name, O_RDWR | O_CREAT, 0666);
            if (shmem_control_input_fd < 0)
            {
                printf("ERROR: Failed calling control shm_open(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            // Size it
            result = ftruncate(shmem_control_input_fd, CONTROL_INPUT_SIZE);
            if (result != 0)
            {
                printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            // Sync
            fsync(shmem_control_input_fd);

            // Close the descriptor
            if (close(shmem_control_input_fd) != 0)
            {
                printf("ERROR: Failed calling close(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }
        }

        // Open the control input shared memory as read-only
        shmem_control_input_fd = shm_open(shmem_control_input_name, O_RDONLY, 0666);
        if (shmem_control_input_fd < 0)
        {
            printf("ERROR: Failed calling precompile RO shm_open(%s) as read-only errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map precompile address space
        if (verbose) gettimeofday(&start_time, NULL);
        void * pControl = mmap((void *)CONTROL_INPUT_ADDR, CONTROL_INPUT_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_control_input_fd, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pControl == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(control_input) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (pControl != (void *)CONTROL_INPUT_ADDR)
        {
            printf("ERROR: Called mmap(control_input) but returned address = %p != 0x%08lx\n", pControl, CONTROL_INPUT_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        shmem_control_input_address = (uint64_t *)pControl;
        precompile_written_address = &shmem_control_input_address[0];
        precompile_exit_address = &shmem_control_input_address[1];
        if (verbose) printf("mmap(control_input) mapped %lu B and returned address %p in %lu us\n", CONTROL_INPUT_SIZE, shmem_control_input_address, duration);

        /******************/
        /* CONTROL OUTPUT */
        /******************/

        // Make sure the precompile results shared memory is deleted
        shm_unlink(shmem_control_output_name);

        // Create the control shared memory
        shmem_control_output_fd = shm_open(shmem_control_output_name, O_RDWR | O_CREAT, 0666);
        if (shmem_control_output_fd < 0)
        {
            printf("ERROR: Failed calling control shm_open(%s) errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Size it
        result = ftruncate(shmem_control_output_fd, CONTROL_OUTPUT_SIZE);
        if (result != 0)
        {
            printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map precompile address space
        if (verbose) gettimeofday(&start_time, NULL);
        pControl = mmap((void *)CONTROL_OUTPUT_ADDR, CONTROL_OUTPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_control_output_fd, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pControl == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(control_output) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (pControl != (void *)CONTROL_OUTPUT_ADDR)
        {
            printf("ERROR: Called mmap(control_output) but returned address = %p != 0x%08lx\n", pControl, CONTROL_OUTPUT_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        shmem_control_output_address = (uint64_t *)pControl;
        precompile_read_address = &shmem_control_output_address[0];
        if (verbose) printf("mmap(control_output) mapped %lu B and returned address %p in %lu us\n", CONTROL_OUTPUT_SIZE, shmem_control_output_address, duration);

        /*************************/
        /* PRECOMPILE SEMAPHORES */
        /*************************/

        // Create the semaphore for precompile results available signal
        assert(strlen(sem_prec_avail_name) > 0);

        sem_unlink(sem_prec_avail_name);

        sem_prec_avail = sem_open(sem_prec_avail_name, O_CREAT | O_EXCL, 0666, 0);
        if (sem_prec_avail == SEM_FAILED)
        {
            printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("sem_open(%s) succeeded sem_prec_avail=%p\n", sem_prec_avail_name, sem_prec_avail);

        // Create the semaphore for precompile results read signal
        assert(strlen(sem_prec_read_name) > 0);

        sem_unlink(sem_prec_read_name);

        sem_prec_read = sem_open(sem_prec_read_name, O_CREAT | O_EXCL, 0666, 0);
        if (sem_prec_read == SEM_FAILED)
        {
            printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_prec_read_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("sem_open(%s) succeeded sem_prec_read=%p\n", sem_prec_read_name, sem_prec_read);
    }

    /*******/
    /* RAM */
    /*******/

    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {

        if (verbose) gettimeofday(&start_time, NULL);
        void * pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED | map_locked_flag, -1, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pRam == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(ram) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pRam != RAM_ADDR)
        {
            printf("ERROR: Called mmap(ram) but returned address = %p != 0x%08lx\n", pRam, RAM_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(ram) mapped %lu B and returned address %p in %lu us\n", RAM_SIZE, pRam, duration);
    }

    /****************/
    /* OUTPUT TRACE */
    /****************/

    // If ROM histogram, configure trace size
    if (gen_method == RomHistogram)
    {
        // Get max PC values for low and high addresses
        uint64_t max_bios_pc = get_max_bios_pc();
        uint64_t max_program_pc = get_max_program_pc();
        assert(max_bios_pc >= 0x1000);
        assert((max_bios_pc & 0x3) == 0);
        assert(max_program_pc >= 0x80000000);

        // Calculate sizes
        bios_size = ((max_bios_pc - 0x1000) >> 2) + 1;
        program_size = max_program_pc - 0x80000000 + 1;
        histogram_size = (4 + 1 + bios_size + 1 + program_size)*8;
        initial_trace_size = ((histogram_size/TRACE_SIZE_GRANULARITY) + 1) * TRACE_SIZE_GRANULARITY;
        trace_size = initial_trace_size;
    }

    // Output trace
    if ((gen_method == MinimalTrace) ||
        (gen_method == RomHistogram) ||
        (gen_method == MainTrace) ||
        (gen_method == Zip) ||
        (gen_method == MemOp) ||
        (gen_method == ChunkPlayerMTCollectMem) ||
        (gen_method == MemReads) ||
        (gen_method == ChunkPlayerMemReadsCollectMain))
    {
        trace_map_initialize();
    }

    /***********************/
    /* INPUT MINIMAL TRACE */
    /***********************/

    // Input MT trace
    if ((gen_method == ChunkPlayerMTCollectMem) || (gen_method == ChunkPlayerMemReadsCollectMain))
    {
        // Create the output shared memory
        shmem_mt_fd = shm_open(shmem_mt_name, O_RDONLY, 0666);
        if (shmem_mt_fd < 0)
        {
            printf("ERROR: Failed calling mt shm_open(%s) errno=%d=%s\n", shmem_mt_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map it to the trace address
#ifdef DEBUG
        gettimeofday(&start_time, NULL);
#endif
        void * pTrace = mmap((void *)TRACE_ADDR, chunk_player_mt_size, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_mt_fd, 0);
#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
#endif
        if (pTrace == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(MT) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pTrace != TRACE_ADDR)
        {
            printf("ERROR: Called mmap(MT) but returned address = %p != 0x%lx\n", pTrace, TRACE_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(MT) returned %p in %lu us\n", pTrace, duration);
    }

    /******************/
    /* SEM CHUNK DONE */
    /******************/

    if (call_chunk_done)
    {
        assert(strlen(sem_chunk_done_name) > 0);

        sem_unlink(sem_chunk_done_name);

        sem_chunk_done = sem_open(sem_chunk_done_name, O_CREAT | O_EXCL, 0666, 0);
        if (sem_chunk_done == SEM_FAILED)
        {
            printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("sem_open(%s) succeeded\n", sem_chunk_done_name);
    }

    /*********************/
    /* SEM SHUTDOWN DONE */
    /*********************/
    
    assert(strlen(sem_shutdown_done_name) > 0);

    sem_unlink(sem_shutdown_done_name);
    
    sem_shutdown_done = sem_open(sem_shutdown_done_name, O_CREAT | O_EXCL, 0666, 0);
    if (sem_shutdown_done == SEM_FAILED)
    {
        printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_shutdown_done_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (verbose) printf("sem_open(%s) succeeded\n", sem_shutdown_done_name);
}

void server_reset_fast (void)
{
    // Reset precompile read address for next emulation
    if (precompile_results_enabled)
    {
        // Set precompile read counter to 0 for next emulation
        *precompile_read_address = 0;

        // Sync control output shared memory so that the writer can see the precompile reads we have
        // done, and thus update the precompile_written_address if needed
        if (msync((void *)shmem_control_output_address, CONTROL_OUTPUT_SIZE, MS_SYNC) != 0) {
            printf("ERROR: server_reset_fast() msync failed for shmem_control_output_address errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }
}

void server_reset_slow (void)
{
    // Reset RAM data for next emulation
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
#ifdef DEBUG
        gettimeofday(&start_time, NULL);
#endif
        memset((void *)RAM_ADDR, 0, RAM_SIZE);
#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        if (verbose) printf("server_reset_slow() memset(ram) in %lu us\n", duration);
#endif
    }
}

void server_reset_trace (void)
{
    // Reset trace header and trace_used_size for next emulation
    if ( (gen_method != ChunkPlayerMTCollectMem) &&
         (gen_method != ChunkPlayerMemReadsCollectMain) &&
         (gen_method != Fast) &&
         (gen_method != RomHistogram) )
    {
        // Reset trace: init output header data
        pOutputTrace[0] = 0x000100; // Version, e.g. v1.0.0 [8]
        pOutputTrace[1] = 1; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
        pOutputTrace[2] = trace_size; // MT allocated size [8] -> to be updated after reallocation
        pOutputTrace[3] = 0; // MT used size [8] -> to be updated after completion
        
        // Reset trace used size
        trace_used_size = 0;
    }
}

void server_run (void)
{
    // If ROM histogram, reset the trace area to 0 for the histogram data since it represents the
    // ROM instruction multiplicity and one of them will be increased at every executed instruction
    if ((gen_method == RomHistogram)) {
        memset((void *)trace_address, 0, trace_size);
    }

#ifdef ASM_CALL_METRICS
    reset_asm_call_metrics();
#endif

    // Init trace header
    server_reset_trace();

    // Sync input shared memory
    if (msync((void *)INPUT_ADDR, MAX_INPUT_SIZE, MS_SYNC) != 0) 
    {
        printf("ERROR: msync failed for shmem_input_address errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    if (precompile_results_enabled)
    {
        // Sync control input shared memory
        if (msync((void *)shmem_control_input_address, CONTROL_INPUT_SIZE, MS_SYNC) != 0) {
            printf("ERROR: msync failed for shmem_control_input_address errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Sync precompile shared memory
        if (msync((void *)shmem_precompile_address, MAX_PRECOMPILE_SIZE, MS_SYNC) != 0) {
            printf("ERROR: msync failed for shmem_precompile_address errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }

    /*******/
    /* ASM */
    /*******/

    // Call emulator assembly code
    gettimeofday(&start_time,NULL);
    if (verbose)
    {
        printf("Before calling emulator_start() trace_address=%lx\n", trace_address);
        fflush(stdout);
        fflush(stderr);
    }
    emulator_start();
    if (verbose)
    {
        printf("After calling emulator_start() trace_address=%lx\n", trace_address);
        fflush(stdout);
        fflush(stderr);
    }
    gettimeofday(&stop_time,NULL);
    assembly_duration = TimeDiff(start_time, stop_time);

    // Reset precompile read address for next emulation
    if (precompile_results_enabled)
    {
        *precompile_read_address = 0;
    }

    uint64_t final_trace_size = MEM_CHUNK_ADDRESS - MEM_TRACE_ADDRESS;
    trace_used_size = final_trace_size + 32;

    if ( metrics )
    {
        uint64_t duration = assembly_duration;
        uint64_t steps = MEM_STEP;
        uint64_t end = MEM_END;
        uint64_t error = MEM_ERROR;
        uint64_t step_duration_ns = steps == 0 ? 0 : (duration * 1000) / steps;
        uint64_t step_tp_sec = duration == 0 ? 0 : steps * 1000000 / duration;
        uint64_t final_trace_size_percentage = (final_trace_size * 100) / trace_size;
        printf("Duration = %lu us, realloc counter = %lu, wait counter = %lu, steps = %lu, step duration = %lu ns, tp = %lu steps/s, trace size = 0x%lx - 0x%lx = %lu B(%lu%% of %lu), end=%lu, error=%lu, max steps=%lu, chunk size=%lu, prec_written=%lu, prec_read=%lu\n",
            duration,
            realloc_counter,
            wait_counter,
            steps,
            step_duration_ns,
            step_tp_sec,
            MEM_CHUNK_ADDRESS,
            MEM_TRACE_ADDRESS,
            final_trace_size,
            final_trace_size_percentage,
            trace_size,
            end,
            error,
            max_steps,
            chunk_size,
            precompile_written_address ? *precompile_written_address : 0,
            precompile_read_address ? *precompile_read_address : 0
        );
        fflush(stdout);
        fflush(stderr);
        if (gen_method == RomHistogram)
        {
            printf("Rom histogram size=%lu\n", histogram_size);
            fflush(stdout);
            fflush(stderr);
        }
    }
    if (MEM_ERROR)
    {
        printf("Emulation ended with error code %lu\n", MEM_ERROR);
        fflush(stdout);
        fflush(stderr);
    }

    // Log output
    if (output)
    {
        unsigned int * pOutput = (unsigned int *)OUTPUT_ADDR;
        unsigned int output_size = 64;
#ifdef DEBUG
        if (verbose)
        {
            printf("Output size=%d\n", output_size);
            fflush(stdout);
            fflush(stderr);
        }
#endif

        for (unsigned int i = 0; i < output_size; i++)
        {
            printf("%08x\n", *pOutput);
            pOutput++;
        }
        fflush(stdout);
        fflush(stderr);
    }

    // Log output for riscof tests
    if (output_riscof)
    {
        unsigned int * pOutput = (unsigned int *)OUTPUT_ADDR;
        unsigned int output_size = *pOutput;
#ifdef DEBUG
        if (verbose)
        {
            printf("Output size=%d\n", output_size);
            fflush(stdout);
            fflush(stderr);
        }
#endif

        for (unsigned int i = 0; i < output_size; i++)
        {
            pOutput++;
            printf("%08x\n", *pOutput);
        }
        fflush(stdout);
        fflush(stderr);
    }

    // Complete output header data
    if ((gen_method == MinimalTrace) ||
        (gen_method == RomHistogram) ||
        (gen_method == Zip) ||
        (gen_method == MainTrace) ||
        (gen_method == MemOp) ||
        (gen_method == MemReads) ||
        (gen_method == ChunkPlayerMemReadsCollectMain))
    {
        uint64_t * pOutput = (uint64_t *)trace_address;
        pOutput[0] = 0x000100; // Version, e.g. v1.0.0 [8]
        pOutput[1] = MEM_ERROR; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
        pOutput[2] = trace_size; // MT allocated size [8]
        //assert(final_trace_size > 32);
        if (gen_method == RomHistogram)
        {
            pOutput[3] = MEM_STEP;
            pOutput[4] = bios_size;
            pOutput[4 + bios_size + 1] = program_size;
        }
        else
        {
            pOutput[3] = trace_used_size; // MT used size [8]
        }
    }

    // Notify client
    if (gen_method == RomHistogram)
    {
        _chunk_done();   
    }


    // Notify the caller that the trace is ready to be consumed
    // if (!is_file)
    // {
    //     result = sem_post(sem_input);
    //     if (result == -1)
    //     {
    //         printf("Failed calling sem_post(%s) errno=%d=%s\n", sem_input_name, errno, strerror(errno));
    //         fflush(stdout);
    //         fflush(stderr);
    //         exit(-1);
    //     }
    // }


#ifdef ASM_CALL_METRICS
    print_asm_call_metrics(assembly_duration);
#endif

    // Log trace
    if (((gen_method == MinimalTrace) || (gen_method == Zip)) && trace)
    {
        log_minimal_trace();
    }
    if ((gen_method == RomHistogram) && trace)
    {
        log_histogram();
    }
    if ((gen_method == MainTrace) && trace)
    {
        log_main_trace();
    }
    if ((gen_method == MemOp) && trace)
    {
        log_mem_op();
    }
    if ((gen_method == MemOp) && save_to_file)
    {
        save_mem_op_to_files();
    }
    if ((gen_method == ChunkPlayerMTCollectMem) && trace)
    {
        log_mem_trace();
    }
    if ((gen_method == MemReads) && trace)
    {
        log_minimal_trace();
    }
    if ((gen_method == ChunkPlayerMemReadsCollectMain) && trace)
    {
        log_chunk_player_main_trace();
    }
}

void server_cleanup (void)
{
    // Cleanup ROM
    int result = munmap((void *)ROM_ADDR, ROM_SIZE);
    if (result == -1)
    {
        printf("ERROR: Failed calling munmap(rom) errno=%d=%s\n", errno, strerror(errno));
    }

    // Cleanup RAM
    result = munmap((void *)RAM_ADDR, RAM_SIZE);
    if (result == -1)
    {
        printf("ERROR: Failed calling munmap(ram) errno=%d=%s\n", errno, strerror(errno));
    }

    // Cleanup INPUT
    result = munmap((void *)INPUT_ADDR, MAX_INPUT_SIZE);
    if (result == -1)
    {
        printf("ERROR: Failed calling munmap(input) errno=%d=%s\n", errno, strerror(errno));
    }
    result = shm_unlink(shmem_input_name);
    if (result == -1)
    {
        printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
    }

    if (precompile_results_enabled && (gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
        // Cleanup PRECOMPILE
        result = munmap((void *)shmem_precompile_address, MAX_PRECOMPILE_SIZE);
        if (result == -1)
        {
            printf("ERROR: Failed calling munmap(precompile) errno=%d=%s\n", errno, strerror(errno));
        }
        result = shm_unlink(shmem_precompile_name);
        if (result == -1)
        {
            printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
        }

        // Cleanup CONTROL
        result = munmap((void *)shmem_control_input_address, CONTROL_INPUT_SIZE);
        if (result == -1)
        {
            printf("ERROR: Failed calling munmap(control_input) errno=%d=%s\n", errno, strerror(errno));
        }
        result = shm_unlink(shmem_control_input_name);
        if (result == -1)
        {
            printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
        }
        result = munmap((void *)shmem_control_output_address, CONTROL_OUTPUT_SIZE);
        if (result == -1)
        {
            printf("ERROR: Failed calling munmap(control_output) errno=%d=%s\n", errno, strerror(errno));
        }
        result = shm_unlink(shmem_control_output_name);
        if (result == -1)
        {
            printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
        }

        // Semaphores cleanup
        result = sem_close(sem_prec_avail);
        if (result == -1)
        {
            printf("ERROR: Failed calling sem_close(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_prec_avail_name);
        if (result == -1)
        {
            printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
        }
        result = sem_close(sem_prec_read);
        if (result == -1)
        {
            printf("ERROR: Failed calling sem_close(%s) errno=%d=%s\n", sem_prec_read_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_prec_read_name);
        if (result == -1)
        {
            printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_prec_read_name, errno, strerror(errno));
        }
    }

    // Cleanup trace
    trace_cleanup();

    // Cleanup chunk done semaphore
    if (call_chunk_done)
    {
        result = sem_close(sem_chunk_done);
        if (result == -1)
        {
            printf("ERROR: Failed calling sem_close(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_chunk_done_name);
        if (result == -1)
        {
            printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        }
    }

    // Post shutdown done semaphore
    result = sem_post(sem_shutdown_done);
    if (result == -1)
    {
        printf("ERROR: Failed calling sem_post(%s) errno=%d=%s\n", sem_shutdown_done_name, errno, strerror(errno));
    }
}