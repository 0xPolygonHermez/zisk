#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <errno.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "constants.hpp"
#include "globals.hpp"
#include "asm_provided.hpp"
#include "log.hpp"

// This file contains trace logging functions that are used only for debugging purposes, to log the
// content of the generated traces in a human-readable format.  These functions are not used by the
// assembly code, and are not optimized for performance.

/*****************/
/* LOG FUNCTIONS */
/*****************/

/* Trace data structure
    [8B] Number of chunks: C

    Chunk 0:
        Start state:
            [8B] pc
            [8B] sp
            [8B] c
            [8B] step
            [8B] register[1]
            …
            [8B] register[31]
            [8B] register[32]
            [8B] register[33]
        Last state:
            [8B] c
        End:
            [8B] end
        Steps:
            [8B] steps = chunk size except for the last chunk
            [8B] mem_reads_size
            [8B] mem_reads[0]
            [8B] mem_reads[1]
            …
            [8B] mem_reads[mem_reads_size - 1]

    Chunk 1:
    …
    Chunk C-1:
    …
*/
void log_minimal_trace(void)
{
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    asm_printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    asm_printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    asm_printf("Allocated size = %lu B\n", pOutput[2]); // Allocated size [8]
    asm_printf("Minimal trace used size = %lu B\n", pOutput[3]); // Minimal trace used size [8]

    asm_printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    asm_printf("Number of chunks=%lu\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        asm_printf("ERROR: Number of chunks is too high=%lu\n", number_of_chunks);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        uint64_t i=0;
        asm_printf("Chunk %lu (@=%p):\n", c, chunk);

        // Log current chunk start state
        asm_printf("\tStart state:\n");
        asm_printf("\t\tpc=0x%lx\n", chunk[i]);
        i++;
        asm_printf("\t\tsp=0x%lx\n", chunk[i]);
        i++;
        asm_printf("\t\tc=0x%lx\n", chunk[i]);
        i++;
        asm_printf("\t\tstep=%lu\n", chunk[i]);
        i++;
        for (uint64_t r=1; r<34; r++)
        {
            asm_printf("\t\treg[%lu]=0x%lx\n", r, chunk[i]);
            i++;
        }

        // Log current chunk last state
        asm_printf("\tEnd state:\n");
        asm_printf("\t\tc=0x%lx\n", chunk[i]);
        i++;
        // Log current chunk end
        asm_printf("\t\tend=%lu\n", chunk[i]);
        i++;
        // Log current chunk steps
        asm_printf("\t\tsteps=%lu\n", chunk[i]);
        i++;

        uint64_t mem_reads_size = chunk[i];
        asm_printf("\t\tmem_reads_size=%lu\n", mem_reads_size);
        i++;
        if (mem_reads_size > 10000000)
        {
            asm_printf("ERROR: Mem reads size is too high=%lu\n", mem_reads_size);
            exit(-1);
        }
        if (trace_trace)
        {
            for (uint64_t m=0; m<mem_reads_size; m++)
            {
                asm_printf("\t\tchunk[%lu].mem_reads[%lu]=%08lx\n", c, m, chunk[i]);
                i++;
            }
        }
        else
        {
            i += mem_reads_size;
        }

        //Set next chunk pointer
        chunk = chunk + i;
    }
    asm_printf("Trace=%p chunk=%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}

void log_histogram(void)
{
    uint64_t *  pOutput = (uint64_t *)TRACE_ADDR;
    asm_printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    asm_printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    asm_printf("Allocated size = %lu B\n", pOutput[2]); // MT allocated size [8]
    asm_printf("Steps = %lu B\n", pOutput[3]); // MT used size [8]

    asm_printf("BIOS histogram:\n");
    uint64_t * trace = (uint64_t *)(TRACE_ADDR + 0x20);

    // BIOS
    uint64_t bios_size = trace[0];
    asm_printf("BIOS size=%lu\n", bios_size);
    if (bios_size > 100000000)
    {
        asm_printf("ERROR: Bios size is too high=%lu\n", bios_size);
        exit(-1);
    }
    if (trace_trace)
    {
        uint64_t * bios = trace + 1;
        for (uint64_t i=0; i<bios_size; i++)
        {
            asm_printf("%lu: pc=0x%lx multiplicity=%lu:\n", i, 0x1000 + (i*4), bios[i] );
        }
    }

    // Program
    uint64_t program_size = trace[bios_size + 1];
    asm_printf("Program size=%lu\n", program_size);
    if (program_size > 100000000)
    {
        asm_printf("ERROR: Program size is too high=%lu\n", program_size);
        exit(-1);
    }
    if (trace_trace)
    {
        uint64_t * program = trace + 1 + bios_size + 1;
        for (uint64_t i=0; i<program_size; i++)
        {
            if (program[i] != 0)
            {
                asm_printf("%lu: pc=0x%lx multiplicity=%lu:\n", i, 0x80000000 + i, program[i]);
            }
        }
    }

    asm_printf("Histogram bios_size=%lu program_size=%lu\n", bios_size, program_size);
}

/* Trace data structure
    [8B] Number of chunks = C

    Chunk 0:
        [8B] mem_trace_size
        [7x8B] mem_trace[0]
        [7x8B] mem_trace[1]
        …
        [7x8B] mem_trace[mem_trace_size - 1]

    Chunk 1:
    …
    Chunk C-1:
    …
*/
void log_main_trace(void)
{
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    asm_printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    asm_printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    asm_printf("Allocated size = %lu B\n", pOutput[2]); // Allocated size [8]
    asm_printf("Main trace used size = %lu B\n", pOutput[3]); // Main trace used size [8]

    asm_printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    asm_printf("Number of chunks=%lu\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        asm_printf("ERROR: Number of chunks is too high=%lu\n", number_of_chunks);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        uint64_t i=0;
        asm_printf("Chunk %lu:\n", c);

        uint64_t main_trace_size = chunk[i];
        asm_printf("\tmem_reads_size=%lu\n", main_trace_size);
        i++;
        main_trace_size /= 7;
        if (main_trace_size > 10000000)
        {
            asm_printf("ERROR: Main_trace size is too high=%lu\n", main_trace_size);
            exit(-1);
        }

        if (trace_trace)
        {
            for (uint64_t m=0; m<main_trace_size; m++)
            {
                asm_printf("\t\tchunk[%lu].main_trace[%lu]=[%lx,%lx,%lx,%lx,%lx,%lx,%lx]\n",
                    c,
                    m,
                    chunk[i],
                    chunk[i+1],
                    chunk[i+2],
                    chunk[i+3],
                    chunk[i+4],
                    chunk[i+5],
                    chunk[i+6]
                );
                i += 7;
            }
        }
        else
        {
            i += main_trace_size*7;
        }

        //Set next chunk pointer
        chunk = chunk + i;
    }
    asm_printf("Trace=%p chunk=%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}

static void buffer2file (const void * buffer_address, size_t buffer_length, const char * file_name)
{
    if (!file_name)
    {
        asm_printf("ERROR: buffer2file() found invalid file_name\n");
        exit(-1);
    }
    if (!buffer_address)
    {
        asm_printf("ERROR: buffer2file() found invalid buffer_address\n");
        exit(-1);
    }

    FILE * file = fopen(file_name, "wb");
    if (!file)
    {
        asm_printf("ERROR: buffer2file() failed calling fopen(%s) errno=%d=%s\n", file_name, errno, strerror(errno));
        exit(-1);
    }

    if (buffer_length > 0)
    {
        size_t bytes_written = fwrite(buffer_address, 1, buffer_length, file);
        if (bytes_written != buffer_length)
        {
            asm_printf("ERROR: buffer2file() failed calling fwrite(%s) buffer_address=%p buffer_length=%zu errno=%d=%s\n", file_name, buffer_address, buffer_length, errno, strerror(errno));
            fclose(file);
            exit(-1);
        }
    }

    if (fclose(file) != 0)
    {
        asm_printf("ERROR: buffer2file() failed calling fclose(%s) errno=%d=%s\n", file_name, errno, strerror(errno));
        exit(-1);
    }
}

/* Memory operations structure
    [8B] Number of chunks = C

    Chunk 0:
        [8b] end
        [8B] mem_op_trace_size
        [8B] mem_op_trace[0]
        [8B] mem_op_trace[1]
        …
        [8B] mem_op_trace[mem_op_trace_size - 1]

    Chunk 1:
    …
    Chunk C-1:
    …
*/
void log_mem_op(void)
{
    // Log header
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    asm_printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    asm_printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    asm_printf("Allocated size = %lu B\n", pOutput[2]); // Allocated size [8]
    asm_printf("Memory operations trace used size = %lu B\n", pOutput[3]); // Main trace used size [8]

    asm_printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    asm_printf("Number of chunks=%lu\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        asm_printf("ERROR: Number of chunks is too high=%lu\n", number_of_chunks);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        uint64_t i=0;
        asm_printf("Chunk %lu:\n", c);

        uint64_t end = chunk[i];
        asm_printf("\tend=%lu\n", end);
        i++;

        uint64_t mem_op_trace_size = chunk[i];
        asm_printf("\tmem_op_trace_size=%lu\n", mem_op_trace_size);
        i++;
        if (mem_op_trace_size > 10000000)
        {
            asm_printf("ERROR: Mem op trace size is too high=%lu\n", mem_op_trace_size);
            exit(-1);
        }

        for (uint64_t m=0; m<mem_op_trace_size; m++)
        {
            uint64_t rest_are_zeros = (chunk[i] >> 49) & 0x1;
            uint64_t write = (chunk[i] >> 48) & 0x1;
            uint64_t width = (chunk[i] >> 32) & 0xF;
            uint64_t address = chunk[i] & 0xFFFFFFFF;
            bool inside_range =
                ((address >= RAM_ADDR) && (address < (RAM_ADDR + RAM_SIZE))) ||
                ((address >= ROM_ADDR) && (address < (ROM_ADDR + ROM_SIZE))) ||
                ((address >= INPUT_ADDR) && (address < (INPUT_ADDR + MAX_INPUT_SIZE)));
            if (trace_trace || !inside_range)
            {
                asm_printf("\t\tchunk[%lu].mem_op_trace[%lu] = %016lx = rest_are_zeros=%lx, write=%lx, width=%lx, address=%lx%s\n",
                    c,
                    m,
                    chunk[i],
                    rest_are_zeros,
                    write,
                    width,
                    address,
                    inside_range ? "" : " ERROR!!!!!!!!!!!!!!"
                );
            }
            i += 1;
        }

        //Set next chunk pointer
        chunk = chunk + i;
    }
    asm_printf("Trace=%p chunk=%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}

/* Memory trace structure (for 1 chunk)
    [8B] mem_trace_size
    [16B] mem_trace[0]
        [8B] mem operacion
            [4B] address (LE)
            [1B] width (1, 2, 4, 8) + write (0, 1) << 4
            [3B] 
    [16B] mem_trace[1]
    …
    [16B] mem_trace[mem_trace_size - 1]
*/
void log_mem_trace(void)
{
    asm_printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)TRACE_ADDR;
    asm_printf("log_mem_trace() trace_address=%p\n", trace);
    uint64_t i=0;
    asm_printf("Version = 0x%06lx\n", trace[0]); // Version, e.g. v1.0.0 [8]
    asm_printf("Exit code = %lu\n", trace[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    asm_printf("Allocated size = %lu B\n", trace[2]); // Allocated size [8]
    asm_printf("Memory operations trace used size = %lu B\n", trace[3]); // Main trace used size [8]
    i += 4;
    uint64_t number_of_entries = trace[i];
    i++;
    asm_printf("Trace size=%lu\n", number_of_entries);

    for (uint64_t m = 0; m < number_of_entries; m++)
    {
        uint64_t addr_step = trace[i];
        i++;

        // addr_step = [@0, @1, @2, @3, width + write<<4, supra_step]        
        uint64_t address = addr_step & 0xFFFFFFFF;
        uint64_t width = (addr_step >> (4*8)) & 0xF;
        uint64_t write = (addr_step >> ((4*8) + 4)) & 0x1;
        uint64_t micro_step = (addr_step >> (5*8)) & 0x3;
        uint64_t incremental_step = (addr_step >> ((5*8) + 2));
        bool address_is_inside_range =
            ((address >= RAM_ADDR) && (address < (RAM_ADDR + RAM_SIZE))) ||
            ((address >= ROM_ADDR) && (address < (ROM_ADDR + ROM_SIZE))) ||
            ((address >= INPUT_ADDR) && (address < (INPUT_ADDR + MAX_INPUT_SIZE)));
        bool width_is_valid = (width == 1) || (width == 2) || (width == 4) || (width == 8);
        bool bError = !(address_is_inside_range && width_is_valid);
        if (trace_trace || bError)
        {
            asm_printf("\tmem_trace[%lu] = %016lx = [inc_step=%lu, u_step=%lu, write=%lx, width=%lx, address=%lx] %s\n",
                m,
                addr_step,
                incremental_step,
                micro_step,
                write,
                width,
                address,
                bError ? " ERROR!!!!!!!!!!!!!!" : ""
            );
        }

        // u-step:
        //   0: a=SRC_MEM
        //   1: b=SRC_MEM or b=SRC_IND
        //   2: precompiled_read
        //   3: c=STORE_MEM, c=STORE_IND or precompiled_write

        bool address_is_aligned = (address & 0x7) == 0;
        uint64_t aligned_address = address & 0xFFFFFFF8;
        uint64_t number_of_read_values = 0;
        uint64_t number_of_write_values = 0;

        switch (micro_step)
        {
            case 0: // a=SRC_MEM
            {
                assert(width == 8);
                if (address_is_aligned)
                {
                    number_of_read_values = 1;
                }
                else
                {
                    number_of_read_values = 2;
                }
                break;
            }
            case 1: // b=SRC_MEM or b=SRC_IND
            {
                if (address_is_aligned)
                {
                    number_of_read_values = 1;
                }
                else
                {
                    if (((address + width - 1) & 0xFFFFFFF8) == aligned_address)
                    {
                        number_of_read_values = 1;
                    }
                    else
                    {
                        number_of_read_values = 2;
                    }
                }
                break;
            }
            case 2: // precompiled_read
            {
                assert(width == 8);
                if (address_is_aligned)
                {
                    number_of_read_values = 1;
                }
                else
                {
                    number_of_read_values = 2;
                }
                break;
            }
            case 3: // c=STORE_MEM, c=STORE_IND or precompiled_write
            {
                if (address_is_aligned && (width == 8))
                {
                    number_of_read_values = 0;
                }
                else
                {
                    if (((address + width - 1) & 0xFFFFFFF8) == aligned_address)
                    {
                        number_of_read_values = 1;
                    }
                    else
                    {
                        number_of_read_values = 2;
                    }
                }
                number_of_write_values = 1;
                break;
            }
        }

        for (uint64_t r = 0; r < number_of_read_values; r++)
        {
            uint64_t value = trace[i];
            i++;
            m++;
            if (trace_trace)
            {
                asm_printf("\t\tread_value[%lu] = 0x%lx\n", i, value);
            }
        }

        for (uint64_t w = 0; w < number_of_write_values; w++)
        {
            uint64_t value = trace[i];
            i++;
            m++;
            if (trace_trace)
            {
                asm_printf("\t\twrite_value[%lu] = 0x%lx\n", i, value);
            }
        }
    }
    asm_printf("Trace=%p number_of_entries=%lu\n", trace, number_of_entries);
}

void save_mem_op_to_files(void)
{
    // Log header
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    asm_printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    asm_printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    asm_printf("Allocated size = %lu B\n", pOutput[2]); // Allocated size [8]
    asm_printf("Memory operations trace used size = %lu B\n", pOutput[3]); // Main trace used size [8]

    asm_printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    asm_printf("Number of chunks=%lu\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        asm_printf("ERROR: Number of chunks is too high=%lu\n", number_of_chunks);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        char file_name[256];
        int file_name_len = snprintf(file_name, sizeof(file_name), "/tmp/mem_count_data_%lu.bin", c);
        if (file_name_len < 0 || (size_t)file_name_len >= sizeof(file_name))
        {
            asm_printf("ERROR: Failed to construct file name for chunk=%lu\n", c);
            exit(-1);
        }

        uint64_t i=0;
        i++; // Skip end
        uint64_t mem_op_trace_size = chunk[i];
        i++;
        if (mem_op_trace_size > 10000000)
        {
            asm_printf("ERROR: Mem op trace size is too high=%lu\n", mem_op_trace_size);
            exit(-1);
        }

        asm_printf("Chunk %lu: file=%s length=%lu\n", c, file_name, mem_op_trace_size);

        buffer2file(&chunk[i], mem_op_trace_size * 8, file_name);

        //Set next chunk pointer: skip [end] and [mem_op_trace_size] headers plus data
        chunk = chunk + mem_op_trace_size + 2;
    }
    asm_printf("Trace=%p chunk=%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}

/* Trace data structure
    [8B] Number of elements

    A series of elements with the following structure:
        [8B] op: instruction opcode
        [8B] a: register a value
        [8B] b: register b value
        [8B] precompiled_memory_address: memory read address of the precompiled input data
*/
void log_chunk_player_main_trace(void)
{
    uint64_t * chunk = (uint64_t *)TRACE_ADDR;
    uint64_t i = 0;

    asm_printf("Version = 0x%06lx\n", chunk[0]); // Version, e.g. v1.0.0 [8]
    asm_printf("Exit code = %lu\n", chunk[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    asm_printf("Allocated size = %lu B\n", chunk[2]); // Allocated size [8]
    asm_printf("Memory operations trace used size = %lu B\n", chunk[3]); // Main trace used size [8]
    i = 4;

    uint64_t mem_reads_size = chunk[i];
    i++;
    asm_printf("mem_reads_size=%lu\n", mem_reads_size);
    if (mem_reads_size > 10000000)
    {
        asm_printf("ERROR: Mem reads size is too high=%lu\n", mem_reads_size);
        exit(-1);
    }
    //if (trace_trace)
    {
        for (uint64_t m=0; m<mem_reads_size; m++)
        {
            uint64_t op = chunk[i];
            if (trace_trace) asm_printf("\tmem_reads[%lu] op=0x%lx\n", m, chunk[i]);
            i++;
            m++;
            if (op > 0xFF)
            {
                asm_printf("ERROR!! Invalid op=%lu=0x%lx\n", op, op);
            }
            if (trace_trace) asm_printf("\tmem_reads[%lu] a=0x%08lx\n", m, chunk[i]);
            i++;
            m++;
            if (trace_trace) asm_printf("\tmem_reads[%lu] b=0x%08lx\n", m, chunk[i]);
            i++;
            m++;
            if (   (op == 0xf1) // Keccak
                || (op == 0xf9) // SHA256
                || (op == 0xf2) // Arith256
                || (op == 0xf3) // Arith256Mod
                || (op == 0xf4) // Secp256k1Add
                || (op == 0xf5) // Secp256k1Dbl
                )
            {
                if (trace_trace) asm_printf("\tmem_reads[%lu] precompiled_address=%08lx\n", m, chunk[i]);
                i++;
                m++;
            }
        }
    }

    asm_printf("Chunk=%p size=%lu\n", chunk, mem_reads_size);
}