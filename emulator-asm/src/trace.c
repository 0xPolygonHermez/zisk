#define _GNU_SOURCE
#include <stdio.h>
#include <sys/mman.h>
#include <errno.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <sys/time.h>
#include <semaphore.h>
#include <fcntl.h>
#include <sys/file.h>
#include <unistd.h>
#include "trace.hpp"
#include "constants.hpp"
#include "globals.hpp"
#include "emu.hpp"

uint64_t next_chunk_id = 0; // Next trace chunk id to be mapped, starting from 0
int trace_chunk_fd[TRACE_NUMBER_OF_CHUNKS]; // File descriptors for each chunk
uint64_t trace_total_mapped_size = 0; // Total mapped trace size

void * trace_get_chunk_address (uint64_t chunk_id)
{
    assert(gen_method != RomHistogram || chunk_id == 0);

    if (chunk_id == 0)
    {
        return (void *)TRACE_ADDR;
    }
    else
    {
        return (void *)(TRACE_ADDR + TRACE_INITIAL_SIZE + ((chunk_id - 1) * TRACE_DELTA_SIZE));
    }
}

uint64_t trace_get_chunk_size (uint64_t chunk_id)
{
    if (gen_method == RomHistogram) {
        assert(chunk_id == 0);
        return trace_size;
    }

    if (chunk_id == 0)
    {
        return TRACE_INITIAL_SIZE;
    }
    else
    {
        return TRACE_DELTA_SIZE;
    }
}

void trace_generate_shmem_chunk_name(char * shmem_chunk_name, size_t shmem_chunk_name_size, uint64_t chunk_id)
{
    int result = snprintf(shmem_chunk_name, shmem_chunk_name_size, "%s_%lu", shmem_output_name, chunk_id);
    if (result < 0 || result >= (int)shmem_chunk_name_size)
    {
        printf("ERROR: trace_generate_shmem_chunk_name() failed to create chunk shared memory name\n");
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
}

void trace_cleanup (void)
{
    // Unmap all mapped chunks
    for (uint64_t chunk_id = 0; chunk_id < next_chunk_id; chunk_id++)
    {
        uint64_t chunk_size = trace_get_chunk_size(chunk_id);
        void * chunk_address = trace_get_chunk_address(chunk_id);
        int result = munmap(chunk_address, chunk_size);
        if (result != 0)
        {
            printf("ERROR: trace_cleanup() failed calling munmap() chunk id=%lu size=%lu B address=0x%lx errno=%d=%s\n", chunk_id, chunk_size, (uint64_t)chunk_address, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Close the chunk shared memory file descriptor
        close(trace_chunk_fd[chunk_id]);
        trace_chunk_fd[chunk_id] = -1;

        // Build the chunk shared memory name
        char shmem_chunk_name[128];
        trace_generate_shmem_chunk_name(shmem_chunk_name, sizeof(shmem_chunk_name), chunk_id);

        // Make sure the chunk shared memory is deleted
        shm_unlink(shmem_chunk_name);
    }

    // Reset next chunk id
    next_chunk_id = 0;
}

void trace_preventive_cleanup (void)
{
    // Unmap all mapped chunks
    for (uint64_t chunk_id = 0; chunk_id < TRACE_NUMBER_OF_CHUNKS; chunk_id++)
    {
        // Build the chunk shared memory name
        char shmem_chunk_name[128];
        trace_generate_shmem_chunk_name(shmem_chunk_name, sizeof(shmem_chunk_name), chunk_id);

        // Make sure the chunk shared memory is deleted
        int result = shm_unlink(shmem_chunk_name);
        if (result != 0)
        {
            break;
        }
        if (verbose) printf("trace_preventive_cleanup() unlinked chunk shared memory %s\n", shmem_chunk_name);
    }
}

void trace_map_next_chunk (void)
{
    // Get the next chunk id, size and address
    uint64_t chunk_id = next_chunk_id;
    if (chunk_id >= TRACE_NUMBER_OF_CHUNKS)
    {
        printf("ERROR: trace_map_next_chunk() exceeded maximum number of chunks %lu\n", TRACE_NUMBER_OF_CHUNKS);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    uint64_t chunk_size = trace_get_chunk_size(chunk_id);
    void * chunk_address = trace_get_chunk_address(chunk_id);

    if (verbose) printf("trace_map_next_chunk() mapping chunk id=%lu size=%lu B address=0x%lx\n", chunk_id, chunk_size, (uint64_t)chunk_address);

    // Build the chunk shared memory name
    char shmem_chunk_name[128];
    trace_generate_shmem_chunk_name(shmem_chunk_name, sizeof(shmem_chunk_name), chunk_id);

    // Make sure the chunk shared memory is deleted
    shm_unlink(shmem_chunk_name);

    // Create the output shared memory
    trace_chunk_fd[chunk_id] = shm_open(shmem_chunk_name, O_RDWR | O_CREAT | O_EXCL, 0666);
    if (trace_chunk_fd[chunk_id] < 0)
    {
        printf("ERROR: trace_map_next_chunk() failed calling trace shm_open(%s) errno=%d=%s\n", shmem_chunk_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Size it
    int result = ftruncate(trace_chunk_fd[chunk_id], chunk_size);
    if (result != 0)
    {
        printf("ERROR: trace_map_next_chunk() failed calling ftruncate(%s) errno=%d=%s\n", shmem_chunk_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Sync
    fsync(trace_chunk_fd[chunk_id]);

    // Map it to the trace address
    if (verbose) gettimeofday(&start_time, NULL);
    void * requested_address;
    if ((gen_method == ChunkPlayerMTCollectMem) || (gen_method == ChunkPlayerMemReadsCollectMain))
    {
        requested_address = 0;
    }
    else
    {
        requested_address = (void *)chunk_address;
    }
    int flags = MAP_SHARED | map_locked_flag;
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
        flags |= MAP_FIXED;
    }
    void * pTrace = mmap(requested_address, chunk_size, PROT_READ | PROT_WRITE, flags, trace_chunk_fd[chunk_id], 0);
    if (verbose)
    {
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
    }
    if (pTrace == MAP_FAILED)
    {
        printf("ERROR: trace_map_next_chunk() failed calling mmap(pTrace) name=%s errno=%d=%s\n", shmem_chunk_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain) && ((uint64_t)pTrace != (uint64_t)requested_address))
    {
        printf("ERROR: trace_map_next_chunk() called mmap(trace) but returned address = %p != 0x%lx\n", pTrace, (uint64_t)requested_address);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (verbose) printf("trace_map_next_chunk() mapped %lu B to %s and returned address %p in %lu us\n", chunk_size, shmem_chunk_name, pTrace, duration);

    // Update total mapped size
    trace_total_mapped_size += chunk_size;

    // Increment next chunk id
    next_chunk_id++;
}

void trace_map_initialize (void)
{
    // Perform preventive cleanup of any leftover shared memory chunks
    trace_preventive_cleanup();

    // Map the first chunk, i.e. chunk 0
    trace_map_next_chunk();

    trace_address = TRACE_ADDR;
    pOutputTrace = (uint64_t *)TRACE_ADDR;
}