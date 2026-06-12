NVCC      ?= nvcc
OUT_DIR   ?= ./build

ROOT_DIR := $(dir $(realpath $(firstword $(MAKEFILE_LIST))))

comma := ,

# GNU make carries 'n' in the first word of MAKEFLAGS under --dry-run;
# parse-time $(shell) side effects (stamp write, object wipe) must not fire.
ifneq ($(findstring n,$(firstword -$(MAKEFLAGS))),)
    MAKE_DRY_RUN := 1
endif

# Canonical arch set for "major" builds: all major architectures since
# Ampere — Ampere/Ada/Hopper/Blackwell-DC/Blackwell-consumer.
# sm_100 = B100/B200/GB200 (datacenter Blackwell); sm_120 = RTX 50xx
# (consumer Blackwell). The two Blackwell lineages are NOT cross-compatible:
# sm_100 has TMEM that sm_120 lacks.
CUDA_MAJOR_ARCHS := 80 86 89 90 100 120

# Arch resolution and the staleness stamp run at parse time (GPU probing,
# stamp writes, stale-object wipes) — skip all of it when every requested
# goal is `clean`, where no flags are needed.
ifeq ($(MAKECMDGOALS),)
    NEED_CUDA_ARCH := 1
else ifneq ($(filter-out clean,$(MAKECMDGOALS)),)
    NEED_CUDA_ARCH := 1
endif

ifdef NEED_CUDA_ARCH
ifdef CUDA_GENCODE_FLAGS
    $(info CUDA_GENCODE_FLAGS defined externally: $(CUDA_GENCODE_FLAGS))
else ifneq ($(strip $(CUDA_ARCHS)),)
    CUDA_ARCHS_LC := $(subst M,m,$(subst A,a,$(subst J,j,$(subst O,o,$(subst R,r,$(strip $(CUDA_ARCHS)))))))
    ifeq ($(CUDA_ARCHS_LC),major)
        CUDA_ARCH_LIST := $(CUDA_MAJOR_ARCHS)
    else
        CUDA_ARCH_LIST := $(strip $(subst $(comma), ,$(CUDA_ARCHS)))
        ifeq ($(CUDA_ARCH_LIST),)
            $(error CUDA_ARCHS is set but contains no architectures — expected integers (e.g. '89', '89,120') or 'major')
        endif
    endif
else ifdef CUDA_ARCH
    # Legacy single-arch support: convert to gencode + PTX
    CUDA_ARCH_NUM := $(subst sm_,,$(CUDA_ARCH))
    CUDA_GENCODE_FLAGS := -gencode arch=compute_$(CUDA_ARCH_NUM),code=$(CUDA_ARCH) \
                          -gencode arch=compute_$(CUDA_ARCH_NUM),code=compute_$(CUDA_ARCH_NUM)
    $(info Converted CUDA_ARCH=$(CUDA_ARCH) to gencode flags)
else
    # Auto-detect from host GPU (single arch)
    CONFIGURE_STATUS := $(shell cd $(ROOT_DIR) && ./detect_cuda_arch.sh >&2; echo $$?)
    ifeq ($(CONFIGURE_STATUS),0)
        include $(ROOT_DIR)CudaArch.mk
        CUDA_ARCH_NUM := $(subst sm_,,$(CUDA_ARCH))
        CUDA_GENCODE_FLAGS := -gencode arch=compute_$(CUDA_ARCH_NUM),code=$(CUDA_ARCH) \
                              -gencode arch=compute_$(CUDA_ARCH_NUM),code=compute_$(CUDA_ARCH_NUM)
        $(info Auto-detected $(CUDA_ARCH), gencode flags set)
    else
        CUDA_ARCH_LIST := $(CUDA_MAJOR_ARCHS)
        $(warning CUDA arch auto-detection failed — building for all major archs: $(CUDA_ARCH_LIST))
    endif
endif

# Gencode construction for the arch-list branches above: SASS per arch + one
# PTX per Blackwell lineage (sm_100-119 datacenter vs the rest), since the
# lineages are not cross-compatible.
ifneq ($(strip $(CUDA_ARCH_LIST)),)
    # Validate with make text functions before anything reaches the shell:
    # a token is numeric iff stripping all digits leaves nothing.
    strip-digits = $(subst 0,,$(subst 1,,$(subst 2,,$(subst 3,,$(subst 4,,$(subst 5,,$(subst 6,,$(subst 7,,$(subst 8,,$(subst 9,,$(1)))))))))))
    CUDA_BAD_ARCH := $(strip $(foreach a,$(CUDA_ARCH_LIST),$(if $(call strip-digits,$(a)),$(a))))
    ifneq ($(CUDA_BAD_ARCH),)
        $(error CUDA_ARCHS contains invalid entry '$(firstword $(CUDA_BAD_ARCH))' — expected integers (e.g. '89', '89,120') or 'major')
    endif
    # Numeric sort + dedup; safe to shell out now, the list is digits-only.
    CUDA_ARCH_LIST := $(shell printf '%s\n' $(CUDA_ARCH_LIST) | sort -n -u)
    CUDA_PTX_ARCHS := $(lastword $(filter 10% 11%,$(CUDA_ARCH_LIST))) $(lastword $(filter-out 10% 11%,$(CUDA_ARCH_LIST)))
    CUDA_GENCODE_FLAGS := $(foreach a,$(CUDA_ARCH_LIST),-gencode arch=compute_$(a),code=sm_$(a)) \
                          $(foreach p,$(CUDA_PTX_ARCHS),-gencode arch=compute_$(p),code=compute_$(p))
    $(info CUDA archs $(CUDA_ARCH_LIST), gencode flags set)
endif
endif # NEED_CUDA_ARCH

NVCCFLAGS := -O3 -std=c++20 $(CUDA_GENCODE_FLAGS) -Xcompiler -fPIC

TARGET := libmemcpp_cu.a
SRCS   := count_and_plan.cu count_and_plan_c.cu
OBJS   := $(addprefix $(OUT_DIR)/, $(SRCS:.cu=.o))

# Staleness stamp: make doesn't track flag changes, so wipe the objects at
# parse time when the resolved gencode flags differ from the stamped ones.
ifdef NEED_CUDA_ARCH
CUDA_ARCH_STAMP := $(OUT_DIR)/.cuda_arch_stamp
LAST_CUDA_FLAGS := $(strip $(shell cat $(CUDA_ARCH_STAMP) 2>/dev/null))
ifneq ($(LAST_CUDA_FLAGS),$(strip $(CUDA_GENCODE_FLAGS)))
    ifndef MAKE_DRY_RUN
        ifneq ($(wildcard $(OUT_DIR)/*.o),)
            $(warning CUDA gencode flags changed — removing stale GPU objects in $(OUT_DIR))
            CUDA_ARCH_CLEAN := $(shell rm -f $(OUT_DIR)/*.o $(OUT_DIR)/$(TARGET))
        endif
        CUDA_ARCH_WRITE := $(shell mkdir -p $(OUT_DIR) && echo '$(strip $(CUDA_GENCODE_FLAGS))' > $(CUDA_ARCH_STAMP))
    endif
endif
endif # NEED_CUDA_ARCH

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
