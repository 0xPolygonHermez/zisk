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


MemCountersBusData *compact_and_save(int chunk, BusDataChunk* chunk_data, int count) {
    int size = sizeof(MemCountersBusData) * count;
    MemCountersBusData *out_data = (MemCountersBusData *)malloc(size);
    for (int i = 0; i < count; ++i) {
        out_data[i].addr = chunk_data[i].data[1];
        out_data[i].flags = chunk_data[i].data[3] + ((chunk_data[i].data[0] - 1) << 16);
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


inline uint32_t addr_to_offset_2(uint32_t addr, uint32_t chunk_id = 0, uint32_t index = 0) {
    switch((uint8_t)((addr >> 24) & 0xFE)) {
        case 0x80: return ((addr - 0x80000000) >> (ADDR_LOW_BITS));
        case 0x82: return ((addr - 0x82000000) >> (ADDR_LOW_BITS)) + ADDR_PAGE_SIZE;
        case 0x84: return ((addr - 0x84000000) >> (ADDR_LOW_BITS)) + 2 * ADDR_PAGE_SIZE;
        case 0x86: return ((addr - 0x86000000) >> (ADDR_LOW_BITS)) + 3 * ADDR_PAGE_SIZE;

        case 0x90: return ((addr - 0x90000000) >> (ADDR_LOW_BITS)) + 4 * ADDR_PAGE_SIZE;
        case 0x92: return ((addr - 0x92000000) >> (ADDR_LOW_BITS)) + 5 * ADDR_PAGE_SIZE;
        case 0x94: return ((addr - 0x94000000) >> (ADDR_LOW_BITS)) + 6 * ADDR_PAGE_SIZE;
        case 0x96: return ((addr - 0x96000000) >> (ADDR_LOW_BITS)) + 7 * ADDR_PAGE_SIZE;

        case 0xA0: return ((addr - 0xA0000000) >> (ADDR_LOW_BITS)) + 8 * ADDR_PAGE_SIZE;
        case 0xA2: return ((addr - 0xA2000000) >> (ADDR_LOW_BITS)) + 9 * ADDR_PAGE_SIZE;
        case 0xA4: return ((addr - 0xA4000000) >> (ADDR_LOW_BITS)) + 10 * ADDR_PAGE_SIZE;
        case 0xA6: return ((addr - 0xA6000000) >> (ADDR_LOW_BITS)) + 11 * ADDR_PAGE_SIZE;
        case 0xA8: return ((addr - 0xA8000000) >> (ADDR_LOW_BITS)) + 12 * ADDR_PAGE_SIZE;
        case 0xAA: return ((addr - 0xAA000000) >> (ADDR_LOW_BITS)) + 13 * ADDR_PAGE_SIZE;
        case 0xAC: return ((addr - 0xAC000000) >> (ADDR_LOW_BITS)) + 14 * ADDR_PAGE_SIZE;
        case 0xAE: return ((addr - 0xAE000000) >> (ADDR_LOW_BITS)) + 15 * ADDR_PAGE_SIZE;

        case 0xB0: return ((addr - 0xB0000000) >> (ADDR_LOW_BITS)) + 16 * ADDR_PAGE_SIZE;
        case 0xB2: return ((addr - 0xB2000000) >> (ADDR_LOW_BITS)) + 17 * ADDR_PAGE_SIZE;
        case 0xB4: return ((addr - 0xB4000000) >> (ADDR_LOW_BITS)) + 18 * ADDR_PAGE_SIZE;
        case 0xB6: return ((addr - 0xB6000000) >> (ADDR_LOW_BITS)) + 19 * ADDR_PAGE_SIZE;
        case 0xB8: return ((addr - 0xB8000000) >> (ADDR_LOW_BITS)) + 20 * ADDR_PAGE_SIZE;
        case 0xBA: return ((addr - 0xBA000000) >> (ADDR_LOW_BITS)) + 21 * ADDR_PAGE_SIZE;
        case 0xBC: return ((addr - 0xBC000000) >> (ADDR_LOW_BITS)) + 22 * ADDR_PAGE_SIZE;
        case 0xBE: return ((addr - 0xBE000000) >> (ADDR_LOW_BITS)) + 23 * ADDR_PAGE_SIZE;

        case 0xC0: return ((addr - 0xC0000000) >> (ADDR_LOW_BITS)) + 24 * ADDR_PAGE_SIZE;
        case 0xC2: return ((addr - 0xC2000000) >> (ADDR_LOW_BITS)) + 25 * ADDR_PAGE_SIZE;
        case 0xC4: return ((addr - 0xC4000000) >> (ADDR_LOW_BITS)) + 26 * ADDR_PAGE_SIZE;
        case 0xC6: return ((addr - 0xC6000000) >> (ADDR_LOW_BITS)) + 27 * ADDR_PAGE_SIZE;
        case 0xC8: return ((addr - 0xC8000000) >> (ADDR_LOW_BITS)) + 28 * ADDR_PAGE_SIZE;
        case 0xCA: return ((addr - 0xCA000000) >> (ADDR_LOW_BITS)) + 29 * ADDR_PAGE_SIZE;
        case 0xCC: return ((addr - 0xCC000000) >> (ADDR_LOW_BITS)) + 30 * ADDR_PAGE_SIZE;
        case 0xCE: return ((addr - 0xCE000000) >> (ADDR_LOW_BITS)) + 31 * ADDR_PAGE_SIZE;

        case 0xD0: return ((addr - 0xD0000000) >> (ADDR_LOW_BITS)) + 32 * ADDR_PAGE_SIZE;
        case 0xD2: return ((addr - 0xD2000000) >> (ADDR_LOW_BITS)) + 33 * ADDR_PAGE_SIZE;
        case 0xD4: return ((addr - 0xD4000000) >> (ADDR_LOW_BITS)) + 34 * ADDR_PAGE_SIZE;
        case 0xD6: return ((addr - 0xD6000000) >> (ADDR_LOW_BITS)) + 35 * ADDR_PAGE_SIZE;
        case 0xD8: return ((addr - 0xD8000000) >> (ADDR_LOW_BITS)) + 36 * ADDR_PAGE_SIZE;
        case 0xDA: return ((addr - 0xDA000000) >> (ADDR_LOW_BITS)) + 37 * ADDR_PAGE_SIZE;
        case 0xDC: return ((addr - 0xDC000000) >> (ADDR_LOW_BITS)) + 38 * ADDR_PAGE_SIZE;
        case 0xDE: return ((addr - 0xDE000000) >> (ADDR_LOW_BITS)) + 39 * ADDR_PAGE_SIZE;
    }
    printf("Error: addr_to_offset: 0x%X (%d:%d)\n", addr, chunk_id, index);
    exit(1);
}

inline uint32_t addr_to_page_2(uint32_t addr, uint32_t chunk_id = 0, uint32_t index = 0) {
    switch((uint8_t)((addr >> 24) & 0xFE)) {
        case 0x80: return 0;
        case 0x82: return 1;
        case 0x84: return 2;
        case 0x86: return 3;
        case 0x90: return 4;
        case 0x92: return 5;
        case 0x94: return 6;
        case 0x96: return 7;
        case 0xA0: return 8;
        case 0xA2: return 9;
        case 0xA4: return 10;
        case 0xA6: return 11;
        case 0xA8: return 12;
        case 0xAA: return 13;
        case 0xAC: return 14;
        case 0xAE: return 15;
        case 0xB0: return 16;
        case 0xB2: return 17;
        case 0xB4: return 18;
        case 0xB6: return 19;
        case 0xB8: return 20;
        case 0xBA: return 21;
        case 0xBC: return 22;
        case 0xBE: return 23;
        case 0xC0: return 24;
        case 0xC2: return 25;
        case 0xC4: return 26;
        case 0xC6: return 27;
        case 0xC8: return 28;
        case 0xCA: return 29;
        case 0xCC: return 30;
        case 0xCE: return 31;
        case 0xD0: return 32;
        case 0xD2: return 33;
        case 0xD4: return 34;
        case 0xD6: return 35;
        case 0xD8: return 36;
        case 0xDA: return 37;
        case 0xDC: return 38;
        case 0xDE: return 39;
    }
    printf("Error: addr_to_page: 0x%X (%d:%d)\n", addr, chunk_id, index);
    exit(1);
}
