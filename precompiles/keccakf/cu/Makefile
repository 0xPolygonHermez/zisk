NVCC      ?= nvcc
OUT_DIR   ?= ./build

ROOT_DIR := $(dir $(realpath $(firstword $(MAKEFILE_LIST))))

# Single-arch build: this is a spike harness, always compiled on the host that
# runs it. Override with CUDA_ARCH=sm_XX to cross-build.
ifndef CUDA_ARCH
    CONFIGURE_STATUS := $(shell cd $(ROOT_DIR) && ./detect_cuda_arch.sh >&2; echo $$?)
    ifeq ($(CONFIGURE_STATUS),0)
        include $(ROOT_DIR)CudaArch.mk
    else
        CUDA_ARCH := sm_89
        $(warning CUDA arch auto-detection failed — defaulting to sm_89)
    endif
endif

CUDA_ARCH_NUM   := $(subst sm_,,$(CUDA_ARCH))
GENCODE         := -gencode arch=compute_$(CUDA_ARCH_NUM),code=$(CUDA_ARCH) \
                   -gencode arch=compute_$(CUDA_ARCH_NUM),code=compute_$(CUDA_ARCH_NUM)

NVCCFLAGS := -O3 -std=c++20 $(GENCODE) -Xcompiler -fPIC -lineinfo

TARGET := libkeccakf_cu.a
SRCS   := keccakf_witness.cu
OBJS   := $(addprefix $(OUT_DIR)/, $(SRCS:.cu=.o))

all: $(OUT_DIR)/$(TARGET)

$(OUT_DIR)/$(TARGET): $(OBJS)
	ar rcs $@ $^

$(OUT_DIR)/%.o: %.cu
	mkdir -p $(OUT_DIR)
	$(NVCC) $(NVCCFLAGS) -c $< -o $@

clean:
	rm -f $(OUT_DIR)/*.o $(OUT_DIR)/$(TARGET)

.PHONY: all clean
