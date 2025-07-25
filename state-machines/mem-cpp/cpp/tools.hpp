#ifndef __TOOLS_HPP__
#define __TOOLS_HPP__
#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
#include <stdint.h>
#include <time.h>
#include <sys/time.h>
#include <string.h>

#include "mem_types.hpp"

inline uint64_t get_usec() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000 + ts.tv_nsec / 1000;
}


inline int32_t load_from_compact_file(const char *path, size_t chunk_id, MemCountersBusData** chunk) {
    char filename[256];
    snprintf(filename, sizeof(filename), "%s/mem_count_data_%ld.bin", path, chunk_id);
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
    int32_t chunk_size = st.st_size / sizeof(MemCountersBusData);
    int32_t size = sizeof(MemCountersBusData) * chunk_size;
    *chunk = (MemCountersBusData *)malloc(size);
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

inline uint32_t count_operations(MemCountersBusData *chunk_data, int count) {
    uint32_t ops = 0;
    uint32_t cops = 0;
    for (int i = 0; i < count; ++i) {
        const uint32_t bytes = chunk_data[i].flags & 0x0F;
        const uint32_t offset = chunk_data[i].addr & 0x07;
        const bool wr = (chunk_data[i].flags & 0x10000) != 0;
        if (offset == 0 && bytes == 8) {
            cops = 1;
        } else if (offset + bytes > 8) {
            if (wr) {
                cops = 4;
            } else {
                cops = 2;
            }
        } else if (wr) {
            cops = 2;
        } else {
            cops = 1;
        }
        ops += cops;
    }
    return ops;
}

#endif