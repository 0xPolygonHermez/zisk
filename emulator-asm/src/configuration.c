#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include "configuration.hpp"
#include "globals.hpp"
#include "asm_provided.hpp"

/*******************************/
/* ARGUMENTS AND CONFIGURATION */
/*******************************/

// To be overwritten by arguments, if provided; otherwise, default values per generation method are used
uint16_t arguments_port = 0;

// Print usage information: valid arguments
void print_usage (void)
{
    printf("Usage: ziskemuasm\n");
    printf("\t-s(server)\n");
    printf("\t-c(client)\n");
    printf("\t-i <input_file>\n");
    printf("\t-p <port_number>\n");
    printf("\t--gen=0|--generate_fast\n");
    printf("\t--gen=1|--generate_minimal_trace\n");
    printf("\t--gen=2|--generate_rom_histogram\n");
    printf("\t--gen=3|--generate_main_trace\n");
    printf("\t--gen=4|--generate_chunks\n");
    printf("\t--gen=6|--generate_zip\n");
    printf("\t--gen=9|--generate_mem_reads\n");
    printf("\t--gen=10|--generate_chunk_player_mem_reads\n");
    printf("\t--chunk <chunk_number>\n");
    printf("\t--shutdown\n");
    printf("\t--mt <number_of_mt_requests>\n");
    printf("\t-o output on\n");
    printf("\t--output_riscof output riscof on\n");
    printf("\t--silent silent on\n");
    printf("\t--shm_prefix <prefix> (default: ZISK)\n");
    printf("\t-m metrics on\n");
    printf("\t-t trace on\n");
    printf("\t-tt trace_trace on\n");
    printf("\t-f(save to file)\n");
    printf("\t-a chunk_address\n");
    printf("\t-v verbose on\n");
    printf("\t-u unlock physical memory in mmap\n");
    printf("\t--share_input_shm share input shared memories\n");
    printf("\t--open_input_shm open existing input shared memories\n");
#ifdef ASM_PRECOMPILE_CACHE
    printf("\t--precompile-cache-store store precompile results in cache file\n");
    printf("\t--precompile-cache-load load precompile results from cache file\n");
#endif
    if (precompile_results_enabled)
    {
        printf("\t-r <precompile_results_file>\n");
    }
    printf("\t--redirect-output-to-file redirect output to file\n");
    printf("\t-h/--help print this\n");
}

// Parse main function arguments and configure global variables accordingly
void parse_arguments(int argc, char *argv[])
{
    strcpy(shm_prefix, "ZISK");
    uint64_t number_of_selected_generation_methods = 0;
    if (argc > 1)
    {
        for (int i = 1; i < argc; i++)
        {
            if (strcmp(argv[i], "-s") == 0)
            {
                server = true;
                continue;
            }
            if (strcmp(argv[i], "-c") == 0)
            {
                client = true;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=0") == 0) || (strcmp(argv[i], "--generate_fast") == 0))
            {
                gen_method = Fast;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=1") == 0) || (strcmp(argv[i], "--generate_minimal_trace") == 0))
            {
                gen_method = MinimalTrace;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=2") == 0) || (strcmp(argv[i], "--generate_rom_histogram") == 0))
            {
                gen_method = RomHistogram;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=3") == 0) || (strcmp(argv[i], "--generate_main_trace") == 0))
            {
                gen_method = MainTrace;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=4") == 0) || (strcmp(argv[i], "--generate_chunks") == 0))
            {
                gen_method = ChunksOnly;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=6") == 0) || (strcmp(argv[i], "--generate_zip") == 0))
            {
                gen_method = Zip;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=7") == 0) || (strcmp(argv[i], "--generate_mem_op") == 0))
            {
                gen_method = MemOp;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=8") == 0) || (strcmp(argv[i], "--generate_chunk_player_mt_collect_mem") == 0))
            {
                gen_method = ChunkPlayerMTCollectMem;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=9") == 0) || (strcmp(argv[i], "--generate_mem_reads") == 0))
            {
                gen_method = MemReads;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=10") == 0) || (strcmp(argv[i], "--generate_chunk_player_mem_reads") == 0))
            {
                gen_method = ChunkPlayerMemReadsCollectMain;
                number_of_selected_generation_methods++;
                continue;
            }
            if (strcmp(argv[i], "-o") == 0)
            {
                output = true;
                continue;
            }
            if (strcmp(argv[i], "--output_riscof") == 0)
            {
                output_riscof = true;
                continue;
            }
            if (strcmp(argv[i], "--silent") == 0)
            {
                silent = true;
                continue;
            }
            if (strcmp(argv[i], "-m") == 0)
            {
                metrics = true;
                continue;
            }
            if (strcmp(argv[i], "-t") == 0)
            {
                trace = true;
                continue;
            }
            if (strcmp(argv[i], "-tt") == 0)
            {
                trace = true;
                trace_trace = true;
                continue;
            }
            if (strcmp(argv[i], "-v") == 0)
            {
                verbose = true;
                //emu_verbose = true;
                continue;
            }
            if (strcmp(argv[i], "-u") == 0)
            {
                map_locked_flag = 0;
                continue;
            }
            if (strcmp(argv[i], "-h") == 0)
            {
                print_usage();
                exit(0);
            }
            if (strcmp(argv[i], "--help") == 0)
            {
                print_usage();
                exit(0);
            }
            if (strcmp(argv[i], "-i") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -i in the last position; please provide input file after it\n");
                    print_usage();
                    exit(-1);
                }
                if (strlen(argv[i]) > 4095)
                {
                    printf("ERROR: Detected argument -i but next argument is too long\n");
                    print_usage();
                    exit(-1);
                }
                strcpy(input_file, argv[i]);
                continue;
            }
            if (strcmp(argv[i], "--shm_prefix") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument --shm_prefix in the last position; please provide shared mem prefix after it\n");
                    print_usage();
                    exit(-1);
                }
                if (strlen(argv[i]) >= MAX_SHM_PREFIX_LENGTH)
                {
                    printf("ERROR: Detected argument --shm_prefix but next argument is too long\n");
                    print_usage();
                    exit(-1);
                }
                strcpy(shm_prefix, argv[i]);
                continue;
            }
            if (strcmp(argv[i], "--chunk") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument --chunk in the last position; please provide chunk number after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                chunk_mask = strtoul(argv[i], &endptr, 10);

                // Check for errors
                if (errno == ERANGE) {
                    printf("ERROR: Chunk number is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argv[i]) {
                    printf("ERROR: No digits found while parsing chunk number\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("ERROR: Extra characters after chunk number: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else if (chunk_mask > MAX_CHUNK_MASK) {
                    printf("ERROR: Invalid chunk number: %lu\n", chunk_mask);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got chunk_mask= %lu\n", chunk_mask);
                }
                continue;
            }
            if (strcmp(argv[i], "--shutdown") == 0)
            {
                do_shutdown = true;
                continue;
            }
            if (strcmp(argv[i], "--mt") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument --mt in the last position; please provide number of MT requests after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                number_of_mt_requests = strtoul(argv[i], &endptr, 10);

                // Check for errors
                if (errno == ERANGE) {
                    printf("ERROR: Number of MT requests is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argv[i]) {
                    printf("ERROR: No digits found while parsing number of MT requests\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("ERROR: Extra characters after number of MT requests: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else if (number_of_mt_requests > 1000000) {
                    printf("ERROR: Invalid number of MT requests: %lu\n", number_of_mt_requests);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got number of MT requests= %lu\n", number_of_mt_requests);
                }
                continue;
            }
            if (strcmp(argv[i], "-p") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -p in the last position; please provide port number after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                uint64_t arguments_port_u64 = strtoul(argv[i], &endptr, 10);
                if (arguments_port_u64 > 0xFFFF)
                {
                    printf("ERROR: Port number is too large, must be at most 65535\n");
                    print_usage();
                    exit(-1);
                }
                arguments_port = arguments_port_u64 & 0xFFFF; // Keep only lower 16 bits, since port numbers are 16 bits

                // Check for errors
                if (errno == ERANGE) {
                    printf("ERROR: Port number is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argv[i]) {
                    printf("ERROR: No digits found while parsing port number\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("ERROR: Extra characters after port number: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got port number= %u\n", arguments_port);
                }
                continue;
            }
            if (strcmp(argv[i], "-f") == 0)
            {
                save_to_file = true;
                continue;
            }
            if (strcmp(argv[i], "-a") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -a in the last position; please provide chunk address after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                char * argument = argv[i];
                if ((argument[0] == '0') && (argument[1] == 'x')) argument += 2;
                chunk_player_address = strtoul(argument, &endptr, 16);

                // Check for errors
                if (errno == ERANGE) {
                    printf("ERROR: Chunk address is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argument) {
                    printf("ERROR: No digits found while parsing chunk address\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("ERROR: Extra characters after chunk address: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got chunk address= %p\n", (void *)chunk_player_address);
                }
                continue;
            }
            if (strcmp(argv[i], "--share_input_shm") == 0)
            {
                share_input_shm = true;
                continue;
            }
            if (strcmp(argv[i], "--open_input_shm") == 0)
            {
                open_input_shm = true;
                continue;
            }
            if (strcmp(argv[i], "--redirect-output-to-file") == 0)
            {
                redirect_output_to_file = true;
                continue;
            }
#ifdef ASM_PRECOMPILE_CACHE
            if (strcmp(argv[i], "--precompile-cache-store") == 0)
            {
                precompile_cache_enabled = true;
                precompile_cache_store_init();
                continue;
            }
            if (strcmp(argv[i], "--precompile-cache-load") == 0)
            {
                precompile_cache_enabled = true;
                precompile_cache_load_init();
                continue;
            }

#endif
            if (precompile_results_enabled && (strcmp(argv[i], "-r") == 0))
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -r in the last position; please provide precompile results file after it\n");
                    print_usage();
                    exit(-1);
                }
                if (strlen(argv[i]) > 4095)
                {
                    printf("ERROR: Detected argument -r but next argument is too long\n");
                    print_usage();
                    exit(-1);
                }
                strcpy(precompile_file_name, argv[i]);
                continue;
            }
            printf("ERROR: parse_arguments() Unrecognized argument: %s\n", argv[i]);
            print_usage();
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }
#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_enabled == false)
    {
        printf("ERROR: parse_arguments() when in precompile cache mode, you need to use an argument: either --precompile-cache-store or --precompile-cache-load\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
#endif

    // Check that only one generation method was selected as an argument
    if (number_of_selected_generation_methods != 1)
    {
        printf("ERROR! parse_arguments() Invalid arguments: select 1 generation method, and only one\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Check that the generation method selected by the process launcher is the same as the one
    // for which the assembly code was generated
    uint64_t asm_gen_method = get_gen_method();
    if (asm_gen_method != gen_method)
    {
        printf("ERROR! parse_arguments() Inconsistency: C generation method is %u but ASM generation method is %lu\n",
            gen_method,
            asm_gen_method);
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Check server/client
    if (server && client)
    {
        printf("ERROR! parse_arguments() Inconsistency: both server and client at the same time is not possible\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (!server && !client)
    {
        printf("ERROR! parse_arguments() Inconsistency: select server or client\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    if (precompile_results_enabled && client && (strlen(precompile_file_name) == 0))
    {
        printf("ERROR! parse_arguments() when in precompile results mode, you need to provide a precompile results file using -r <precompile_results_file>\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
}

// Configure global variables based on generation method and other arguments
void configure (void)
{
    // Select configuration based on generation method
    switch (gen_method)
    {
        case Fast:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_FT_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_FT_control_output");
            strcpy(shmem_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_input_name, "_input");
            else
                strcat(shmem_input_name, "_FT_input");
            if (precompile_results_enabled)
            {
                strcpy(shmem_precompile_name, shm_prefix);
                if (share_input_shm)
                    strcat(shmem_precompile_name, "_precompile");
                else
                    strcat(shmem_precompile_name, "_FT_precompile");
                strcpy(sem_prec_avail_name, shm_prefix);
                strcat(sem_prec_avail_name, "_FT_prec_avail");
                strcpy(sem_prec_read_name, shm_prefix);
                strcat(sem_prec_read_name, "_FT_prec_read");
            }
            else
            {
                strcpy(shmem_precompile_name, "");
                strcpy(sem_prec_avail_name, "");
                strcpy(sem_prec_read_name, "");
            }
            strcpy(shmem_output_name, "");
            strcpy(sem_chunk_done_name, "");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_FT_shutdown_done");
            strcpy(sem_input_avail_name, shm_prefix);
            strcat(sem_input_avail_name, "_FT_input_avail");
            strcpy(shmem_mt_name, "");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_FT");
            port = 23120;
            break;
        }
        case MinimalTrace:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_MT_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_MT_control_output");
            strcpy(shmem_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_input_name, "_input");
            else
                strcat(shmem_input_name, "_MT_input");
            if (precompile_results_enabled)
            {
                strcpy(shmem_precompile_name, shm_prefix);
                if (share_input_shm)
                    strcat(shmem_precompile_name, "_precompile");
                else
                    strcat(shmem_precompile_name, "_MT_precompile");
                strcpy(sem_prec_avail_name, shm_prefix);
                strcat(sem_prec_avail_name, "_MT_prec_avail");
                strcpy(sem_prec_read_name, shm_prefix);
                strcat(sem_prec_read_name, "_MT_prec_read");
            }
            else
            {
                strcpy(shmem_precompile_name, "");
                strcpy(sem_prec_avail_name, "");
                strcpy(sem_prec_read_name, "");
            }
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_MT_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_MT_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_MT_shutdown_done");
            strcpy(sem_input_avail_name, shm_prefix);
            strcat(sem_input_avail_name, "_MT_input_avail");
            strcpy(shmem_mt_name, "");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_MT");
            call_chunk_done = true;
            port = 23115;
            break;
        }
        case RomHistogram:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_RH_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_RH_control_output");
            strcpy(shmem_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_input_name, "_input");
            else
                strcat(shmem_input_name, "_RH_input");
            if (precompile_results_enabled)
            {
                strcpy(shmem_precompile_name, shm_prefix);
                if (share_input_shm)
                    strcat(shmem_precompile_name, "_precompile");
                else
                    strcat(shmem_precompile_name, "_RH_precompile");
                strcpy(sem_prec_avail_name, shm_prefix);
                strcat(sem_prec_avail_name, "_RH_prec_avail");
                strcpy(sem_prec_read_name, shm_prefix);
                strcat(sem_prec_read_name, "_RH_prec_read");
            }
            else
            {
                strcpy(shmem_precompile_name, "");
                strcpy(sem_prec_avail_name, "");
                strcpy(sem_prec_read_name, "");
            }
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_RH_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_RH_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_RH_shutdown_done");
            strcpy(sem_input_avail_name, shm_prefix);
            strcat(sem_input_avail_name, "_RH_input_avail");
            strcpy(shmem_mt_name, "");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_RH");
            call_chunk_done = true;
            port = 23116;
            break;
        }
        case MainTrace:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_MA_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_MA_control_output");
            strcpy(shmem_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_input_name, "_input");
            else
                strcat(shmem_input_name, "_MA_input");
            if (precompile_results_enabled)
            {
                strcpy(shmem_precompile_name, shm_prefix);
                if (share_input_shm)
                    strcat(shmem_precompile_name, "_precompile");
                else
                    strcat(shmem_precompile_name, "_MA_precompile");
                strcpy(sem_prec_avail_name, shm_prefix);
                strcat(sem_prec_avail_name, "_MA_prec_avail");
                strcpy(sem_prec_read_name, shm_prefix);
                strcat(sem_prec_read_name, "_MA_prec_read");
            }
            else
            {
                strcpy(shmem_precompile_name, "");
                strcpy(sem_prec_avail_name, "");
                strcpy(sem_prec_read_name, "");
            }
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_MA_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_MA_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_MA_shutdown_done");
            strcpy(sem_input_avail_name, shm_prefix);
            strcat(sem_input_avail_name, "_MA_input_avail");
            strcpy(shmem_mt_name, "");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_MA");
            call_chunk_done = true;
            port = 23118;
            break;
        }
        case ChunksOnly:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_CH_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_CH_control_output");
            strcpy(shmem_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_input_name, "_input");
            else
                strcat(shmem_input_name, "_CH_input");
            strcpy(shmem_precompile_name, "");
            strcpy(sem_prec_avail_name, "");
            strcpy(sem_prec_read_name, "");
            strcpy(sem_input_avail_name, "");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_CH_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_CH_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_CH_shutdown_done");
            strcpy(shmem_mt_name, "");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_CH");
            call_chunk_done = true;
            port = 23115;
            break;
        }
        // case BusOp:
        // {
        //     strcpy(shmem_input_name, "ZISKBO_input");
        //     strcpy(shmem_output_name, "ZISKBO_output");
        //     strcpy(sem_chunk_done_name, "ZISKBO_chunk_done");
        //     chunk_done = true;
        //     port = 23115;
        //     break;
        // }
        case Zip:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_ZP_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_ZP_control_output");
            strcpy(shmem_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_input_name, "_input");
            else
                strcat(shmem_input_name, "_ZP_input");
            if (precompile_results_enabled)
            {
                strcpy(shmem_precompile_name, shm_prefix);
                if (share_input_shm)
                    strcat(shmem_precompile_name, "_precompile");
                else
                    strcat(shmem_precompile_name, "_ZP_precompile");
                strcpy(sem_prec_avail_name, shm_prefix);
                strcat(sem_prec_avail_name, "_ZP_prec_avail");
                strcpy(sem_prec_read_name, shm_prefix);
                strcat(sem_prec_read_name, "_ZP_prec_read");
            }
            else
            {
                strcpy(shmem_precompile_name, "");
                strcpy(sem_prec_avail_name, "");
                strcpy(sem_prec_read_name, "");
            }
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_ZP_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_ZP_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_ZP_shutdown_done");
            strcpy(sem_input_avail_name, shm_prefix);
            strcat(sem_input_avail_name, "_ZP_input_avail");
            strcpy(shmem_mt_name, "");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_ZP");
            call_chunk_done = true;
            port = 23115;
            break;
        }
        case MemOp:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_MO_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_MO_control_output");
            strcpy(shmem_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_input_name, "_input");
            else
                strcat(shmem_input_name, "_MO_input");
            if (precompile_results_enabled)
            {
                strcpy(shmem_precompile_name, shm_prefix);
                if (share_input_shm)
                    strcat(shmem_precompile_name, "_precompile");
                else
                    strcat(shmem_precompile_name, "_MO_precompile");
                strcpy(sem_prec_avail_name, shm_prefix);
                strcat(sem_prec_avail_name, "_MO_prec_avail");
                strcpy(sem_prec_read_name, shm_prefix);
                strcat(sem_prec_read_name, "_MO_prec_read");
            }
            else
            {
                strcpy(shmem_precompile_name, "");
                strcpy(sem_prec_avail_name, "");
                strcpy(sem_prec_read_name, "");
            }
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_MO_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_MO_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_MO_shutdown_done");
            strcpy(sem_input_avail_name, shm_prefix);
            strcat(sem_input_avail_name, "_MO_input_avail");
            strcpy(shmem_mt_name, "");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_MO");
            call_chunk_done = true;
            port = 23117;
            break;
        }
        case ChunkPlayerMTCollectMem:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_CM_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_CM_control_output");
            strcpy(shmem_input_name, "");
            strcpy(shmem_precompile_name, "");
            strcpy(sem_prec_avail_name, "");
            strcpy(sem_prec_read_name, "");
            strcpy(sem_input_avail_name, "");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_CM_output");
            strcpy(sem_chunk_done_name, "");
            strcpy(sem_shutdown_done_name, "");
            strcpy(shmem_mt_name, shm_prefix);
            strcat(shmem_mt_name, "_MT_output");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_CM");
            call_chunk_done = false;
            port = 23119;
            break;
        }
        case MemReads:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_MT_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_MT_control_output");
            strcpy(shmem_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_input_name, "_input");
            else
                strcat(shmem_input_name, "_MT_input");
            if (precompile_results_enabled)
            {
                strcpy(shmem_precompile_name, shm_prefix);
                if (share_input_shm)
                    strcat(shmem_precompile_name, "_precompile");
                else
                    strcat(shmem_precompile_name, "_MT_precompile");
                strcpy(sem_prec_avail_name, shm_prefix);
                strcat(sem_prec_avail_name, "_MT_prec_avail");
                strcpy(sem_prec_read_name, shm_prefix);
                strcat(sem_prec_read_name, "_MT_prec_read");
            }
            else
            {
                strcpy(shmem_precompile_name, "");
                strcpy(sem_prec_avail_name, "");
                strcpy(sem_prec_read_name, "");
            }
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_MT_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_MT_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_MT_shutdown_done");
            strcpy(sem_input_avail_name, shm_prefix);
            strcat(sem_input_avail_name, "_MT_input_avail");
            strcpy(shmem_mt_name, "");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_MT");
            call_chunk_done = true;
            port = 23115;
            break;
        }
        case ChunkPlayerMemReadsCollectMain:
        {
            strcpy(shmem_control_input_name, shm_prefix);
            if (share_input_shm)
                strcat(shmem_control_input_name, "_control_input");
            else
                strcat(shmem_control_input_name, "_CA_control_input");
            strcpy(shmem_control_output_name, shm_prefix);
            strcat(shmem_control_output_name, "_CA_control_output");
            strcpy(shmem_input_name, "");
            strcpy(shmem_precompile_name, "");
            strcpy(sem_prec_avail_name, "");
            strcpy(sem_prec_read_name, "");
            strcpy(sem_input_avail_name, "");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_CA_output");
            strcpy(sem_chunk_done_name, "");
            strcpy(sem_shutdown_done_name, "");
            strcpy(shmem_mt_name, shm_prefix);
            strcat(shmem_mt_name, "_MT_output");
            strcpy(file_lock_name, "/tmp/");
            strcat(file_lock_name, shm_prefix);
            strcat(file_lock_name, ".lock");
            strcpy(log_name, shm_prefix);
            strcat(log_name, "_CA");
            call_chunk_done = false;
            port = 23120;
            break;
        }
        default:
        {
            printf("ERROR: configure() Invalid gen_method = %u\n", gen_method);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }

    if (precompile_results_enabled && (gen_method == ChunkPlayerMTCollectMem || gen_method == ChunkPlayerMemReadsCollectMain))
    {
        printf("ERROR: configure() precompile results enabled is not compatible with generation method %u\n", gen_method);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    if (arguments_port != 0)
    {
        port = arguments_port;
    }

    if (verbose)
    {
        printf("ziskemuasm configuration:\n");
        printf("\tgen_method=%u\n", gen_method);
        printf("\tshm_prefix=%s\n", shm_prefix);
        printf("\tfile_lock_name=%s\n", file_lock_name);
        printf("\tlog_name=%s\n", log_name);
        printf("\tport=%u\n", port);
        printf("\tcall_chunk_done=%u\n", call_chunk_done);
        printf("\tchunk_size=%lu\n", chunk_size);
        printf("\tshmem_control_input=%s\n", shmem_control_input_name);
        printf("\tshmem_control_output=%s\n", shmem_control_output_name);
        printf("\tshmem_input=%s\n", shmem_input_name);
        printf("\tshmem_precompile=%s\n", shmem_precompile_name);
        printf("\tshmem_output=%s\n", shmem_output_name);
        printf("\tshmem_mt=%s\n", shmem_mt_name);
        printf("\tsem_chunk_done=%s\n", sem_chunk_done_name);
        printf("\tsem_shutdown_done=%s\n", sem_shutdown_done_name);
        printf("\tsem_prec_avail=%s\n", sem_prec_avail_name);
        printf("\tsem_prec_read=%s\n", sem_prec_read_name);
        printf("\tsem_input_avail=%s\n", sem_input_avail_name);
        printf("\tmap_locked_flag=%d\n", map_locked_flag);
        printf("\toutput=%u\n", output);
        printf("\tprecompile_results_enabled=%u\n", precompile_results_enabled);
        printf("\toutput_riscof=%u\n", output_riscof);
    }
}