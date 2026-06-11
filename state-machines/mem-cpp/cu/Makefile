NVCC      ?= nvcc

ROOT_DIR := $(dir $(realpath $(firstword $(MAKEFILE_LIST))))

ifdef CUDA_GENCODE_FLAGS
    $(info CUDA_GENCODE_FLAGS defined externally: $(CUDA_GENCODE_FLAGS))
else ifdef CUDA_ARCH
    CUDA_ARCH_NUM := $(subst sm_,,$(CUDA_ARCH))
    CUDA_GENCODE_FLAGS := -gencode arch=compute_$(CUDA_ARCH_NUM),code=$(CUDA_ARCH) \
                          -gencode arch=compute_$(CUDA_ARCH_NUM),code=compute_$(CUDA_ARCH_NUM)
    $(info Converted CUDA_ARCH=$(CUDA_ARCH) to gencode flags)
else
    INFO := $(shell cd $(ROOT_DIR) && ./detect_cuda_arch.sh)
    include $(ROOT_DIR)CudaArch.mk
    CUDA_ARCH_NUM := $(subst sm_,,$(CUDA_ARCH))
    CUDA_GENCODE_FLAGS := -gencode arch=compute_$(CUDA_ARCH_NUM),code=$(CUDA_ARCH) \
                          -gencode arch=compute_$(CUDA_ARCH_NUM),code=compute_$(CUDA_ARCH_NUM)
    $(info Auto-detected $(CUDA_ARCH), gencode flags set)
endif

NVCCFLAGS := -O3 -std=c++20 $(CUDA_GENCODE_FLAGS) -Xcompiler -fPIC

TARGET := libmemcpp_cu.a
SRCS   := count_and_plan.cu count_and_plan_c.cu
OBJS   := $(addprefix $(OUT_DIR)/, $(SRCS:.cu=.o))

# Recompile when any header in this dir or the shared cpp/ header changes.
HEADERS := $(wildcard *.cuh) $(wildcard ../cpp/*.hpp)

all: $(OUT_DIR)/$(TARGET)

$(OUT_DIR)/$(TARGET): $(OBJS)
	ar rcs $@ $^

$(OUT_DIR)/%.o: %.cu $(HEADERS)
	mkdir -p $(OUT_DIR)
	$(NVCC) $(NVCCFLAGS) -c $< -o $@

clean:
	rm -f $(OUT_DIR)/*.o $(OUT_DIR)/$(TARGET)

.PHONY: all clean
