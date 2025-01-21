#ifndef _CUDA_UTILS_H_
#define _CUDA_UTILS_H_

#include <stddef.h>
#include <stdint.h>

void alloc_pinned_mem(uint64_t n);

void alloc_pinned_mem_per_device(uint64_t n);

uint64_t* get_pinned_mem();

void free_pinned_mem();

void warmup_all_gpus();

#endif      // _CUDA_UTILS_H_
