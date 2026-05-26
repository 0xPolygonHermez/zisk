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
#include <signal.h>
#include <setjmp.h>
#include "server.hpp"
#include "globals.hpp"
#include "asm_provided.hpp"
#include "trace_logs.hpp"
#include "trace.hpp"
#include "emu.hpp"
#include "c_provided.hpp"
#include "log.hpp"

/****************************/
/* EMULATION FAULT RECOVERY */
/****************************/
// Catch synchronous signals raised while running the JIT-translated assembly
// (a guest panic ends in `unimp`/OOB access) and longjmp back to a safe point
// in `server_run` instead of letting the kernel kill the long-running service.
// `sigsetjmp(env, 1)` saves the signal mask so the caught signal isn't left
// blocked after recovery; the handler runs on `sigaltstack` because the
// emulator swaps `rsp` to a guest stack via `MEM_RSP` mid-run.

static sigjmp_buf emulation_jmp_buf;
static volatile sig_atomic_t in_emulation = 0;
static volatile sig_atomic_t caught_signal = 0;
static char emulation_alt_stack[1 << 17]; // 128 KB

static void on_emulation_signal(int sig)
{
    if (in_emulation)
    {
        in_emulation = 0;
        caught_signal = (sig_atomic_t)sig;
        siglongjmp(emulation_jmp_buf, 1);
    }
    // Faulted outside emulation — no recovery point; restore default and re-raise.
    signal(sig, SIG_DFL);
    raise(sig);
}

// Write a minimal "end" chunk at MEM_CHUNK_ADDRESS so the parent's reader
// sees `end=1` and exits its chunk loop cleanly. Used on signal recovery to
// avoid the parent's 10 s chunk-wait timeout. Layouts must match AsmMOChunk
// / AsmMTChunk in emulator-asm/asm-runner/src/asm_mo.rs and asm_mt.rs:
//   MemOp: { end, mem_ops_size }                                — 2 u64s, end @ 0
//   MT-style: pc,sp,c,step, regs[33], last_c,end,steps,mem_reads_size — 41 u64s, end @ 38
static void write_abort_chunk(void)
{
    bool is_mo = (gen_method == MemOp);
    size_t n_words = is_mo ? 2 : 41;
    size_t end_idx = is_mo ? 0 : 38;

    uint64_t * chunk = (uint64_t *)MEM_CHUNK_ADDRESS;
    memset(chunk, 0, n_words * sizeof(uint64_t));
    chunk[end_idx] = 1;
    MEM_CHUNK_ADDRESS += n_words * sizeof(uint64_t);
}

void server_signal_handler(void)
{
    stack_t alt_stack = {
        .ss_sp   = emulation_alt_stack,
        .ss_size = sizeof(emulation_alt_stack),
    };
    if (sigaltstack(&alt_stack, NULL) != 0)
    {
        asm_printf("WARNING: sigaltstack failed errno=%d=%s\n", errno, strerror(errno));
    }

    struct sigaction sa = {
        .sa_handler = on_emulation_signal,
        .sa_flags   = SA_ONSTACK | SA_NODEFER,
    };
    sigemptyset(&sa.sa_mask);

    int signals[] = { SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGABRT };
    for (size_t i = 0; i < sizeof(signals) / sizeof(signals[0]); ++i)
    {
        if (sigaction(signals[i], &sa, NULL) != 0)
        {
            asm_printf("WARNING: sigaction(sig=%d) failed errno=%d=%s\n",
                       signals[i], errno, strerror(errno));
        }
    }
}

/**********/
/* SERVER */
/**********/

// Huge pages setup:
//
// # Check current huge page status
// cat /proc/meminfo | grep -i huge
//
// # Temporarily reserve 20 huge pages (2MB each)
// echo 20 | sudo tee /proc/sys/vm/nr_hugepages
//
// # Make permanent
// echo "vm.nr_hugepages=20" | sudo tee -a /etc/sysctl.conf

//#define USE_HUGE_PAGES

// ROM histogram
uint64_t histogram_size = 0;
uint64_t rom_length = 0;

// Shutdown done semaphore: notifies the caller when a shutdown has been processed
sem_t * sem_shutdown_done = NULL;

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
        // Get the start time
        if (verbose) gettimeofday(&start_time, NULL);

        if (create_internal_shm)
        {
            // If we are not reusing an existing shared memory, we be more strict with the
            // permissions during the emulation, saving the time to initialize it, done only once.
            // First, the ROM shared memory is created and initialized (zero + ROM init data).
            // Then, the sared memory is reopened as read-only, containing the proper constant data
            // values, and preventing the assembly code from modifying it.

            // Make sure the rom shared memory is deleted
            shm_unlink(shmem_rom_name);

            // Create the rom shared memory
            shmem_rom_fd = shm_open(shmem_rom_name, O_RDWR | O_CREAT | O_EXCL, 0666);
            if (shmem_rom_fd < 0)
            {
                asm_printf("ERROR: Failed creating rom shm_open(%s) as read-write errno=%d=%s\n", shmem_rom_name, errno, strerror(errno));
                exit(-1);
            }

            // Size it
            result = ftruncate(shmem_rom_fd, ROM_SIZE);
            if (result != 0)
            {
                asm_printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_rom_name, errno, strerror(errno));
                exit(-1);
            }

            // Sync
            fsync(shmem_rom_fd);

            // Map as read-write for writing the initialization data
#ifdef USE_HUGE_PAGES
            void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag | MAP_HUGETLB, shmem_rom_fd, 0);
            if (pRom == MAP_FAILED)
            {
                asm_printf("ERROR: Failed calling mmap(rom) with huge pages errno=%d=%s\n", errno, strerror(errno));
                pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_rom_fd, 0);
            }
#else
            void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_rom_fd, 0);
#endif
            if (pRom == MAP_FAILED)
            {
                asm_printf("ERROR: Failed calling mmap(rom) errno=%d=%s\n", errno, strerror(errno));
                exit(-1);
            }
            if ((uint64_t)pRom != ROM_ADDR)
            {
                asm_printf("ERROR: Called mmap(rom) but returned address = %p != 0x%lx\n", pRom, ROM_ADDR);
                exit(-1);
            }

            // Initialize the ROM content by calling the assembly code, which writes it to the
            // shared memory
            write_ro_init_data();

            // Sync
            fsync(shmem_rom_fd);

            // Unmap the ROM memory since we will remap it later as read-only (if create_internal_shm is false) or with the same permissions (if create_internal_shm is true)
            if (munmap(pRom, ROM_SIZE) != 0)
            {
                asm_printf("ERROR: Failed calling munmap(rom) errno=%d=%s\n", errno, strerror(errno));
                exit(-1);
            }

            // Close the descriptor since we don't need it anymore after initializing the content
            close(shmem_rom_fd);
            shmem_rom_fd = -1;

            // Reopen the rom shared memory as read-only for mapping it
            shmem_rom_fd = shm_open(shmem_rom_name, O_RDONLY, 0666);
            if (shmem_rom_fd < 0)
            {
                asm_printf("ERROR: Failed reopening rom RO shm_open(%s) as read-only errno=%d=%s\n", shmem_rom_name, errno, strerror(errno));
                exit(-1);
            }

            // Remap as read-only for the assembly code, which should not modify it
#ifdef USE_HUGE_PAGES
            pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag | MAP_HUGETLB, shmem_rom_fd, 0);
            if (pRom == MAP_FAILED)
            {
                asm_printf("ERROR: Failed calling mmap(rom) with huge pages errno=%d=%s\n", errno, strerror(errno));
                pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_rom_fd, 0);
            }
#else
            pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_rom_fd, 0);
#endif
            if (pRom == MAP_FAILED)
            {
                asm_printf("ERROR: Failed calling mmap(rom) errno=%d=%s\n", errno, strerror(errno));
                exit(-1);
            }
            if ((uint64_t)pRom != ROM_ADDR)
            {
                asm_printf("ERROR: Called mmap(rom) but returned address = %p != 0x%lx\n", pRom, ROM_ADDR);
                exit(-1);
            }
        }
        else
        {
            // If we are reusing an existing shared memory, we have to map it as read-write and
            // initialize the ROM content before every emulation.

            // Open the rom shared memory
            shmem_rom_fd = shm_open(shmem_rom_name, O_RDWR, 0666);
            if (shmem_rom_fd < 0)
            {
                asm_printf("ERROR: Failed opening rom RW shm_open(%s) as read-write errno=%d=%s\n", shmem_rom_name, errno, strerror(errno));
                exit(-1);
            }

            // Map as read-write for writing the initialization data before every emulation
#ifdef USE_HUGE_PAGES
            void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag | MAP_HUGETLB, shmem_rom_fd, 0);
            if (pRom == MAP_FAILED)
            {
                asm_printf("ERROR: Failed calling mmap(rom) with huge pages errno=%d=%s\n", errno, strerror(errno));
                pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_rom_fd, 0);
            }
#else
            void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_rom_fd, 0);
#endif
            if (pRom == MAP_FAILED)
            {
                asm_printf("ERROR: Failed calling mmap(rom) errno=%d=%s\n", errno, strerror(errno));
                exit(-1);
            }
            if ((uint64_t)pRom != ROM_ADDR)
            {
                asm_printf("ERROR: Called mmap(rom) but returned address = %p != 0x%lx\n", pRom, ROM_ADDR);
                exit(-1);
            }

            // Close the descriptor since we don't need it anymore after mapping
            close(shmem_rom_fd);
            shmem_rom_fd = -1;

            // Initialize data by calling the assembly code
            write_ro_init_data();
        }

        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
            asm_printf("mmap(rom) mapped %lu B and returned address %p in %lu us\n", ROM_SIZE, (void *)ROM_ADDR, duration);
        }
    }

    /*********/
    /* INPUT */
    /*********/

    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
        // Get the start time
        if (verbose) gettimeofday(&start_time, NULL);

        if (create_input_shm)
        {
            // Make sure the input shared memory is deleted
            shm_unlink(shmem_input_name);

            // Create the input shared memory
            shmem_input_fd = shm_open(shmem_input_name, O_RDWR | O_CREAT | O_EXCL, 0666);
            if (shmem_input_fd < 0)
            {
                asm_printf("ERROR: Failed calling input RW shm_open(%s) as read-write errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
                exit(-1);
            }

            // Size it
            result = ftruncate(shmem_input_fd, MAX_INPUT_SIZE);
            if (result != 0)
            {
                asm_printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
                exit(-1);
            }

            // Sync
            fsync(shmem_input_fd);

            // Close the descriptor
            if (close(shmem_input_fd) != 0)
            {
                asm_printf("ERROR: Failed calling close(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
                exit(-1);
            }
        }

        // Open the input shared memory as read-only
        shmem_input_fd = shm_open(shmem_input_name, O_RDONLY, 0666);
        if (shmem_input_fd < 0)
        {
            asm_printf("ERROR: Failed calling input RO shm_open(%s) as read-only errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            exit(-1);
        }

        // Map input address space
#ifdef USE_HUGE_PAGES
        void * pInput = mmap((void *)INPUT_ADDR, MAX_INPUT_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag | MAP_HUGETLB, shmem_input_fd, 0);
        if (pInput == MAP_FAILED)
        {
            asm_printf("ERROR: Failed calling mmap(input) with huge pages errno=%d=%s\n", errno, strerror(errno));
            pInput = mmap((void *)INPUT_ADDR, MAX_INPUT_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_input_fd, 0);
        }
#else
        void * pInput = mmap((void *)INPUT_ADDR, MAX_INPUT_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_input_fd, 0);
#endif
        if (pInput == MAP_FAILED)
        {
            asm_printf("ERROR: Failed calling mmap(input) errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
        if ((uint64_t)pInput != INPUT_ADDR)
        {
            asm_printf("ERROR: Called mmap(pInput) but returned address = %p != 0x%lx\n", pInput, INPUT_ADDR);
            exit(-1);
        }
        
        // Report duration
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
            asm_printf("mmap(input) mapped %lu B and returned address %p in %lu us\n", MAX_INPUT_SIZE, pInput, duration);
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

        // Get the start time
        if (verbose) gettimeofday(&start_time, NULL);

        if (create_input_shm)
        {
            // Make sure the precompile results shared memory is deleted
            shm_unlink(shmem_precompile_name);

            // Create the precompile results shared memory
            shmem_precompile_fd = shm_open(shmem_precompile_name, O_RDWR | O_CREAT, 0666);
            if (shmem_precompile_fd < 0)
            {
                asm_printf("ERROR: Failed calling precompile shm_open(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
                exit(-1);
            }

            // Size it
            result = ftruncate(shmem_precompile_fd, MAX_PRECOMPILE_SIZE);
            if (result != 0)
            {
                asm_printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
                exit(-1);
            }

            // Sync
            fsync(shmem_precompile_fd);

            // Close the descriptor
            if (close(shmem_precompile_fd) != 0)
            {
                asm_printf("ERROR: Failed calling close(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
                exit(-1);
            }
        }

        // Open the precompile shared memory as read-only
        shmem_precompile_fd = shm_open(shmem_precompile_name, O_RDONLY, 0666);
        if (shmem_precompile_fd < 0)
        {
            asm_printf("ERROR: Failed calling precompile RO shm_open(%s) as read-only errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
            exit(-1);
        }

        // Map precompile address space
        void * pPrecompile = mmap(NULL, MAX_PRECOMPILE_SIZE, PROT_READ, MAP_SHARED | map_locked_flag, shmem_precompile_fd, 0);
        if (pPrecompile == MAP_FAILED)
        {
            asm_printf("ERROR: Failed calling mmap(precompile) errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
        shmem_precompile_address = pPrecompile;
        precompile_results_address = (uint64_t *)pPrecompile;
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
            asm_printf("mmap(precompile) mapped %lu B and returned address %p in %lu us\n", MAX_PRECOMPILE_SIZE, precompile_results_address, duration);
        }

        /**********************************/
        /* PRECOMPILE AVAILABLE SEMAPHORE */
        /**********************************/

        // Get the start time
        if (verbose) gettimeofday(&start_time, NULL);

        // Create the semaphore for precompile results available signal
        assert(strlen(sem_prec_avail_name) > 0);

        sem_unlink(sem_prec_avail_name);

        sem_prec_avail = sem_open(sem_prec_avail_name, O_CREAT | O_EXCL, 0666, 0);
        if (sem_prec_avail == SEM_FAILED)
        {
            asm_printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
            exit(-1);
        }

        // Report duration
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
            asm_printf("sem_open(%s) succeeded sem_prec_avail=%p in %lu us\n", sem_prec_avail_name, sem_prec_avail, duration);
        }

        /*****************************/
        /* PRECOMPILE READ SEMAPHORE */
        /*****************************/

        // Get the start time
        if (verbose) gettimeofday(&start_time, NULL);

        // Create the semaphore for precompile results read signal
        assert(strlen(sem_prec_read_name) > 0);

        sem_unlink(sem_prec_read_name);

        sem_prec_read = sem_open(sem_prec_read_name, O_CREAT | O_EXCL, 0666, 0);
        if (sem_prec_read == SEM_FAILED)
        {
            asm_printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_prec_read_name, errno, strerror(errno));
            exit(-1);
        }
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
            asm_printf("sem_open(%s) succeeded sem_prec_read=%p in %lu us\n", sem_prec_read_name, sem_prec_read, duration);
        }
    }

    /*****************/
    /* CONTROL INPUT */
    /*****************/

    // Get the start time
    if (verbose) gettimeofday(&start_time, NULL);

    if (create_input_shm)
    {
        // Make sure the precompile results shared memory is deleted
        shm_unlink(shmem_control_input_name);

        // Create the control shared memory
        shmem_control_input_fd = shm_open(shmem_control_input_name, O_RDWR | O_CREAT, 0666);
        if (shmem_control_input_fd < 0)
        {
            asm_printf("ERROR: Failed calling control shm_open(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
            exit(-1);
        }

        // Size it
        result = ftruncate(shmem_control_input_fd, CONTROL_INPUT_SIZE);
        if (result != 0)
        {
            asm_printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
            exit(-1);
        }

        // Sync
        fsync(shmem_control_input_fd);

        // Close the descriptor
        if (close(shmem_control_input_fd) != 0)
        {
            asm_printf("ERROR: Failed calling close(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
            exit(-1);
        }
    }

    // Open the control input shared memory as read-only
    shmem_control_input_fd = shm_open(shmem_control_input_name, O_RDONLY, 0666);
    if (shmem_control_input_fd < 0)
    {
        asm_printf("ERROR: Failed calling precompile RO shm_open(%s) as read-only errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
        exit(-1);
    }

    // Map precompile address space
    void * pControl = mmap((void *)CONTROL_INPUT_ADDR, CONTROL_INPUT_SIZE, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_control_input_fd, 0);
    if (pControl == MAP_FAILED)
    {
        asm_printf("ERROR: Failed calling mmap(control_input) errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }
    if (pControl != (void *)CONTROL_INPUT_ADDR)
    {
        asm_printf("ERROR: Called mmap(control_input) but returned address = %p != 0x%08lx\n", pControl, CONTROL_INPUT_ADDR);
        exit(-1);
    }
    shmem_control_input_address = (uint64_t *)pControl;
    precompile_written_address = &shmem_control_input_address[0];
    precompile_exit_address = &shmem_control_input_address[1];
    input_written_address = &shmem_control_input_address[2];
    precompile_reset_address = &shmem_control_input_address[3];

    // Report duration
    if (verbose)
    {
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        asm_printf("mmap(control_input) mapped %lu B and returned address %p in %lu us\n", CONTROL_INPUT_SIZE, shmem_control_input_address, duration);
    }

    /******************/
    /* CONTROL OUTPUT */
    /******************/

    // Get the start time
    if (verbose) gettimeofday(&start_time, NULL);

    if (create_output_shm)
    {
        // Make sure the precompile results shared memory is deleted
        shm_unlink(shmem_control_output_name);

        // Create the control shared memory
        shmem_control_output_fd = shm_open(shmem_control_output_name, O_RDWR | O_CREAT, 0666);
        if (shmem_control_output_fd < 0)
        {
            asm_printf("ERROR: Failed creating control shm_open(%s) errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
            exit(-1);
        }

        // Size it
        result = ftruncate(shmem_control_output_fd, CONTROL_OUTPUT_SIZE);
        if (result != 0)
        {
            asm_printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
            exit(-1);
        }

        // Sync
        fsync(shmem_control_output_fd);
    }
    else
    {
        // Open the control output shared memory as read-write
        shmem_control_output_fd = shm_open(shmem_control_output_name, O_RDWR, 0666);
        if (shmem_control_output_fd < 0)
        {
            asm_printf("ERROR: Failed opening control shm_open(%s) as read-write errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
            exit(-1);
        }
    }

    // Map precompile address space
    pControl = mmap((void *)CONTROL_OUTPUT_ADDR, CONTROL_OUTPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_control_output_fd, 0);
    if (pControl == MAP_FAILED)
    {
        asm_printf("ERROR: Failed calling mmap(control_output) errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }
    if (pControl != (void *)CONTROL_OUTPUT_ADDR)
    {
        asm_printf("ERROR: Called mmap(control_output) but returned address = %p != 0x%08lx\n", pControl, CONTROL_OUTPUT_ADDR);
        exit(-1);
    }
    shmem_control_output_address = (uint64_t *)pControl;
    precompile_read_address = &shmem_control_output_address[0];
    waiting_for_precompile_address = &shmem_control_output_address[1];
    waiting_for_input_address = &shmem_control_output_address[2];

    // Report duration
    if (verbose)
    {
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        asm_printf("mmap(control_output) mapped %lu B and returned address %p in %lu us\n", CONTROL_OUTPUT_SIZE, shmem_control_output_address, duration);
    }

    /*******/
    /* RAM */
    /*******/

    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
        // Get the start time
        if (verbose) gettimeofday(&start_time, NULL);

        if (create_internal_shm)
        {
            // Make sure the ram shared memory is deleted
            shm_unlink(shmem_ram_name);

            // Create the ram shared memory
            shmem_ram_fd = shm_open(shmem_ram_name, O_RDWR | O_CREAT | O_EXCL, 0666);
            if (shmem_ram_fd < 0)
            {
                asm_printf("ERROR: Failed creating ram shm_open(%s) as read-write errno=%d=%s\n", shmem_ram_name, errno, strerror(errno));
                exit(-1);
            }

            // Size it
            result = ftruncate(shmem_ram_fd, RAM_SIZE);
            if (result != 0)
            {
                asm_printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_ram_name, errno, strerror(errno));
                exit(-1);
            }

            // Sync
            fsync(shmem_ram_fd);
        }
        else
        {
            // Open the ram shared memory as read-write
            shmem_ram_fd = shm_open(shmem_ram_name, O_RDWR, 0666);
            if (shmem_ram_fd < 0)
            {
                asm_printf("ERROR: Failed opening ram shm_open(%s) as read-write errno=%d=%s\n", shmem_ram_name, errno, strerror(errno));
                exit(-1);
            }
        }

        // Map it to the ram address
#ifdef USE_HUGE_PAGES
        void * pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag | MAP_HUGETLB, shmem_ram_fd, 0);
        if (pRam == MAP_FAILED)
        {
            asm_printf("ERROR: Failed calling mmap(ram) with huge pages errno=%d=%s\n", errno, strerror(errno));
            pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_ram_fd, 0);
        }
#else
        void * pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_ram_fd, 0);
#endif
        if (pRam == MAP_FAILED)
        {
            asm_printf("ERROR: Failed calling mmap(ram) errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
        if ((uint64_t)pRam != RAM_ADDR)
        {
            asm_printf("ERROR: Called mmap(ram) but returned address = %p != 0x%08lx\n", pRam, RAM_ADDR);
            exit(-1);
        }
        
        // Close the descriptor since we don't need it anymore after mapping
        close(shmem_ram_fd);
        shmem_ram_fd = -1;

        // Report duration
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
            asm_printf("mmap(ram) mapped %lu B and returned address %p in %lu us\n", RAM_SIZE, pRam, duration);
        }
    }

    /****************/
    /* OUTPUT TRACE */
    /****************/

    // If ROM histogram, configure trace size
    if (gen_method == RomHistogram)
    {
        // Get rom length, i.e. number of instructions
        rom_length = get_rom_length();

        // Calculate histogram size
        histogram_size = (4 + 1 + rom_length)*8;
        if (histogram_size > TRACE_INITIAL_SIZE_RH)
        {
            asm_printf("ERROR: ROM histogram size %lu is larger than the trace initial size RH %lu\n", histogram_size, TRACE_INITIAL_SIZE_RH);
            exit(-1);
        }
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
        // Get the start time
        if (verbose) gettimeofday(&start_time, NULL);

        // Create the output shared memory
        shmem_mt_fd = shm_open(shmem_mt_name, O_RDONLY, 0666);
        if (shmem_mt_fd < 0)
        {
            asm_printf("ERROR: Failed calling mt shm_open(%s) errno=%d=%s\n", shmem_mt_name, errno, strerror(errno));
            exit(-1);
        }

        // Map it to the trace address
        void * pTrace = mmap((void *)TRACE_ADDR, chunk_player_mt_size, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_mt_fd, 0);
        if (pTrace == MAP_FAILED)
        {
            asm_printf("ERROR: Failed calling mmap(MT) errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
        if ((uint64_t)pTrace != TRACE_ADDR)
        {
            asm_printf("ERROR: Called mmap(MT) but returned address = %p != 0x%lx\n", pTrace, TRACE_ADDR);
            exit(-1);
        }

        // Report duration
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
            asm_printf("mmap(MT) returned %p in %lu us\n", pTrace, duration);
        }
    }

    /******************/
    /* SEM CHUNK DONE */
    /******************/

    if (call_chunk_done)
    {
        // Get the start time
        if (verbose) gettimeofday(&start_time, NULL);

        assert(strlen(sem_chunk_done_name) > 0);

        // Delete the semaphore if it already exists since we are going to create it with O_CREAT | O_EXCL and want to make sure it succeeds
        sem_unlink(sem_chunk_done_name);

        // Create the semaphore for chunk done signal
        sem_chunk_done = sem_open(sem_chunk_done_name, O_CREAT | O_EXCL, 0666, 0);
        if (sem_chunk_done == SEM_FAILED)
        {
            asm_printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
            exit(-1);
        }

        // Report duration
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
            asm_printf("sem_open(%s) succeeded in %lu us\n", sem_chunk_done_name, duration);
        }
    }

    /*********************/
    /* SEM SHUTDOWN DONE */
    /*********************/

    // Get the start time
    if (verbose) gettimeofday(&start_time, NULL);
    
    assert(strlen(sem_shutdown_done_name) > 0);

    // Delete the semaphore if it already exists since we are going to create it with O_CREAT | O_EXCL and want to make sure it succeeds
    sem_unlink(sem_shutdown_done_name);
    
    // Create the semaphore for shutdown done signal
    sem_shutdown_done = sem_open(sem_shutdown_done_name, O_CREAT | O_EXCL, 0666, 0);
    if (sem_shutdown_done == SEM_FAILED)
    {
        asm_printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_shutdown_done_name, errno, strerror(errno));
        exit(-1);
    }

    // Report duration
    if (verbose)
    {
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        asm_printf("sem_open(%s) succeeded in %lu us\n", sem_shutdown_done_name, duration);
    }

    /***********************/
    /* SEM INPUT AVAILABLE */
    /***********************/

    // Get the start time
    if (verbose) gettimeofday(&start_time, NULL);

    assert(strlen(sem_input_avail_name) > 0);

    // Delete the semaphore if it already exists since we are going to create it with O_CREAT | O_EXCL and want to make sure it succeeds
    sem_unlink(sem_input_avail_name);

    // Create the semaphore for input available signal
    sem_input_avail = sem_open(sem_input_avail_name, O_CREAT | O_EXCL, 0666, 0);
    if (sem_input_avail == SEM_FAILED)
    {
        asm_printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_input_avail_name, errno, strerror(errno));
        exit(-1);
    }

    // Report duration
    if (verbose)
    {
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        asm_printf("sem_open(%s) succeeded in %lu us\n", sem_input_avail_name, duration);
    }
}

void server_reset_fast (void)
{
#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif
    // Set control output counters to 0 for next emulation
    *precompile_read_address = 0;
    *waiting_for_precompile_address = 0;
    *waiting_for_input_address = 0;

    // Sync control output shared memory so that the writer can see the counters now
    if (msync((void *)shmem_control_output_address, CONTROL_OUTPUT_SIZE, MS_SYNC) != 0)
    {
        asm_printf("ERROR: server_reset_fast() msync failed for shmem_control_output_address errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }
#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    if (verbose) asm_printf("server_reset_fast() msync(shmem_control_output_address) in %lu us\n", duration);
#endif
}

void server_reset_slow (void)
{
    // Reset RAM and ROM data for next emulation
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
#ifdef DEBUG
        gettimeofday(&start_time, NULL);
#endif
        // Reset RAM data
        memset((void *)RAM_ADDR, 0, RAM_SIZE);
        write_rw_init_data();

        // Reset ROM data only if we are sharing the ROM memory with other assembly processes, i.e.
        // if other programs are executed using the same ROM, with different content.
        // If we created it from scratch, we initialized it with the correct content and reopen it
        // as read-only, so we don't need to reset it again.
        if (!create_internal_shm)
        {
            memset((void *)ROM_ADDR, 0, ROM_SIZE);
            write_ro_init_data();
        }
        
#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        if (verbose) asm_printf("server_reset_slow() memset(ram), write_rw_init_data(), and memset(rom) in %lu us\n", duration);
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

    // Reset flags
    if (wait_flag)
    {
        *waiting_for_precompile_address = 0;
        *waiting_for_input_address = 0;
    }
    
    // Reset counters
    wait_prec_avail_counter = 0;
    wait_input_avail_counter = 0;
    print_pc_counter = 0;
}

void server_run (void)
{
    // If ROM histogram, reset the trace area to 0 for the histogram data since it represents the
    // ROM instruction multiplicity and one of them will be increased at every executed instruction
    if ((gen_method == RomHistogram))
    {
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
        asm_printf("ERROR: msync failed for shmem_input_address errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }

    if (precompile_results_enabled)
    {
        // Sync control input shared memory
        if (msync((void *)shmem_control_input_address, CONTROL_INPUT_SIZE, MS_SYNC) != 0) {
            asm_printf("ERROR: msync failed for shmem_control_input_address errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }

        // Sync precompile shared memory
        if (msync((void *)shmem_precompile_address, MAX_PRECOMPILE_SIZE, MS_SYNC) != 0) {
            asm_printf("ERROR: msync failed for shmem_precompile_address errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
    }

    /*******/
    /* ASM */
    /*******/

    // Run the emulator under a sigsetjmp guard — see signal handler block above.
    // On fault we set MEM_ERROR, write a sentinel "end" chunk so the parent's
    // chunk-wait wakes immediately (instead of timing out), and fall through.
    // The post-emulation code publishes MEM_ERROR in the trace header; the
    // response then carries result=1 back to the parent, which reports the
    // error.
    gettimeofday(&start_time,NULL);
    if (verbose) asm_printf("Before calling emu_start() trace_address=%lx\n", trace_address);
    caught_signal = 0;
    bool emulation_aborted = false;
    if (sigsetjmp(emulation_jmp_buf, 1) == 0)
    {
        in_emulation = 1;
        emu_start();
        in_emulation = 0;
    }
    else
    {
        in_emulation = 0;
        MEM_ERROR = (uint64_t)caught_signal;
        emulation_aborted = true;
        asm_printf("WARNING: caught signal %d during emulation, aborting run\n",
                   (int)caught_signal);
    }
    if (verbose) asm_printf("After calling emu_start() trace_address=%lx\n", trace_address);
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
        asm_printf("Duration = %lu us, realloc counter = %lu, wait prec counter = %lu, wait input counter = %lu, steps = %lu, step duration = %lu ns, tp = %lu steps/s, trace size = 0x%lx - 0x%lx = %lu B(%lu%% of %lu), end=%lu, error=%lu, max steps=%lu, chunk size=%lu, prec_written=%lu, prec_read=%lu\n",
            duration,
            realloc_counter,
            wait_prec_avail_counter,
            wait_input_avail_counter,
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
            asm_printf("Rom histogram size=%lu\n", histogram_size);
        }
    }
    if (MEM_ERROR)
    {
        asm_printf("Emulation ended with error code %lu\n", MEM_ERROR);
    }

    // Log output
    if (output)
    {
        unsigned int * pOutput = (unsigned int *)OUTPUT_ADDR;
        unsigned int output_size = 64;
#ifdef DEBUG
        if (verbose)
        {
            asm_printf("Output size=%d\n", output_size);
        }
#endif

        for (unsigned int i = 0; i < output_size; i++)
        {
            asm_raw_printf("%08x\n", *pOutput);
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
            asm_printf("Output size=%d\n", output_size);
        }
#endif

        for (unsigned int i = 0; i < output_size; i++)
        {
            pOutput++;
            asm_raw_printf("%08x\n", *pOutput);
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
            pOutput[4] = rom_length;
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
    else if (emulation_aborted && call_chunk_done)
    {
        write_abort_chunk();
        _chunk_done();
    }


    // Notify the caller that the trace is ready to be consumed
    // if (!is_file)
    // {
    //     result = sem_post(sem_input);
    //     if (result == -1)
    //     {
    //         asm_printf("Failed calling sem_post(%s) errno=%d=%s\n", sem_input_name, errno, strerror(errno));
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
        asm_printf("ERROR: Failed calling munmap(rom) errno=%d=%s\n", errno, strerror(errno));
    }
    if (delete_internal_shm)
    {
        result = shm_unlink(shmem_rom_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_rom_name, errno, strerror(errno));
        }
    }

    // Cleanup RAM
    result = munmap((void *)RAM_ADDR, RAM_SIZE);
    if (result == -1)
    {
        asm_printf("ERROR: Failed calling munmap(ram) errno=%d=%s\n", errno, strerror(errno));
    }
    if (delete_internal_shm)
    {
        result = shm_unlink(shmem_ram_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_ram_name, errno, strerror(errno));
        }
    }

    // Cleanup INPUT
    result = munmap((void *)INPUT_ADDR, MAX_INPUT_SIZE);
    if (result == -1)
    {
        asm_printf("ERROR: Failed calling munmap(input) errno=%d=%s\n", errno, strerror(errno));
    }
    if (delete_input_shm)
    {
        result = shm_unlink(shmem_input_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
        }
    }

    if (precompile_results_enabled && (gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
        // Cleanup PRECOMPILE
        result = munmap((void *)shmem_precompile_address, MAX_PRECOMPILE_SIZE);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling munmap(precompile) errno=%d=%s\n", errno, strerror(errno));
        }
        if (delete_input_shm)
        {
            result = shm_unlink(shmem_precompile_name);
            if (result == -1)
            {
                asm_printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
            }
        }

        // Semaphores cleanup
        result = sem_close(sem_prec_avail);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling sem_close(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_prec_avail_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
        }
        result = sem_close(sem_prec_read);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling sem_close(%s) errno=%d=%s\n", sem_prec_read_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_prec_read_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_prec_read_name, errno, strerror(errno));
        }
        result = sem_close(sem_input_avail);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling sem_close(%s) errno=%d=%s\n", sem_input_avail_name, errno, strerror(errno));
        }
    }

    // Cleanup CONTROL
    result = munmap((void *)shmem_control_input_address, CONTROL_INPUT_SIZE);
    if (result == -1)
    {
        asm_printf("ERROR: Failed calling munmap(control_input) errno=%d=%s\n", errno, strerror(errno));
    }
    if (!wait_flag && delete_input_shm)
    {
        result = shm_unlink(shmem_control_input_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
        }
    }
    result = munmap((void *)shmem_control_output_address, CONTROL_OUTPUT_SIZE);
    if (result == -1)
    {
        asm_printf("ERROR: Failed calling munmap(control_output) errno=%d=%s\n", errno, strerror(errno));
    }
    if (!wait_flag && delete_output_shm)
    {
        result = shm_unlink(shmem_control_output_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
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
            asm_printf("ERROR: Failed calling sem_close(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_chunk_done_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        }
    }

    // Cleanup input available semaphore
    result = sem_unlink(sem_input_avail_name);
    if (result == -1)
    {
        asm_printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_input_avail_name, errno, strerror(errno));
    }

    // Post shutdown done semaphore
    if (just_create_all_shm)
    {
        result = sem_unlink(sem_shutdown_done_name);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_shutdown_done_name, errno, strerror(errno));
        }
    }
    else{
        result = sem_post(sem_shutdown_done);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling sem_post(%s) errno=%d=%s\n", sem_shutdown_done_name, errno, strerror(errno));
        }
    }
}