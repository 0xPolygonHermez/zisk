#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
#include <vector>
#include <thread>
#include <iostream>
#include <string.h>
#include <sys/time.h>
#include <vector>

#include "mem_types.hpp"

#define MEM_BUS_DATA_SIZE 7 // Replace with your actual size

typedef struct {
    uint64_t data[MEM_BUS_DATA_SIZE];
} BusDataChunk;
/*
typedef struct {
    uint32_t addr;
    uint32_t flags;
} MemCountersBusData;
*/
int load_from_file(size_t chunk_id, BusDataChunk** chunk) {
    char filename[256];
    snprintf(filename, sizeof(filename), "../bus_data/mem_%ld.bin", chunk_id);
    int fd = open(filename, O_RDONLY);
    if (fd < 0) {
        return -1;
    }
    struct stat st;
    if (fstat(fd, &st) < 0) {
        perror("Error getting file size");
        close(fd);
        return -1;
    }
    int chunk_size = st.st_size / sizeof(BusDataChunk);
    int size = sizeof(BusDataChunk) * chunk_size;
    *chunk = (BusDataChunk *)malloc(size);
    if (*chunk == NULL) {
        perror("Error allocating memory");
        close(fd);
        return -1;
    }
    ssize_t bytes_read = read(fd, *chunk, size);
    if (bytes_read < 0) {
        perror("Error reading file");
        free(*chunk);
        close(fd);
        return -1;
    }
    close(fd);
    if (bytes_read != size) {
        fprintf(stderr, "Warning: Read %zd bytes, expected %d bytes\n", bytes_read, size);
    }
    return chunk_size;
}


// Converts a chunk of raw bus data (MemBusData: op, addr, step, bytes, ...) into the compact file
// that load_from_compact_file reads: one light-form MOPS header per access (mops_format.hpp), the
// same stream that MemCounters::save_to_compact_file writes.
uint64_t *compact_and_save(int chunk, BusDataChunk* chunk_data, int count) {
    const uint64_t MOPS_TAG = 1ULL << 63;
    int size = sizeof(uint64_t) * count;
    uint64_t *out_data = (uint64_t *)malloc(size);
    for (int i = 0; i < count; ++i) {
        const uint64_t addr = chunk_data[i].data[1] & MOPS_ADDR_MASK;
        const bool is_write = chunk_data[i].data[0] == 2;   // MEMORY_STORE_OP
        uint64_t mode, no_value;
        if (is_write) {
            mode = (chunk_data[i].data[3] & 0x0F) | MOPS_WRITE_FLAG;
            no_value = 1;
        } else if (addr == INPUT_ADDR) {   // the free-input word
            mode = MOPS_READ_8_VALUE;
            no_value = 1;
        } else {
            mode = chunk_data[i].data[3] & 0x0F;
            no_value = 0;
        }
        out_data[i] = MOPS_TAG | addr | (mode << MOPS_MODE_SHIFT) | (no_value << MOPS_NO_VALUE_BIT);
    }
    char filename[256];
    snprintf(filename, sizeof(filename), "../bus_data/mem_count_data/mem_count_data_%d.bin", chunk);
    int fd = open(filename, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        perror("Error opening file for writing");
        free(out_data);
        return NULL;
    }
    ssize_t bytes_written = write(fd, out_data, size);
    if (bytes_written < 0) {
        perror("Error writing file");
        free(out_data);
        close(fd);
        return NULL;
    }
    close(fd);
    return out_data;
}

void convert_to_compact(void){
    BusDataChunk *chunk_data = NULL;
    int chunk_size;
    int chunks = 0;
    int tot_chunks = 0;
    while (chunks < MAX_CHUNKS && (chunk_size = load_from_file(chunks, &chunk_data)) >=0) {
        printf("converting chunk %d with size %d\n", chunks, chunk_size);
        free(compact_and_save(chunks, chunk_data, chunk_size));
        free(chunk_data);
        chunks++;
        tot_chunks += chunk_size;
    }
    printf("chunks: %d  tot_chunks: %d\n", chunks, tot_chunks);
}