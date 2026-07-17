TRACE_TARGET=NONE

# Debug build flags
ifeq ($(dbg),1)
	# CFLAGS = -O0 -g -D DEBUG -no-pie -ggdb -fno-inline
	CFLAGS = -g -D DEBUG -no-pie -fno-pie
	ASMFLAGS = -g --noexecstack
else
	CFLAGS = -O3 -no-pie -fno-pie
	ASMFLAGS = --noexecstack
endif

ifeq ($(TRACE_TARGET),MT)
    CFLAGS += -DTRACE_TARGET_MT
    ASMFLAGS += --defsym TRACE_TARGET=1
else ifeq ($(TRACE_TARGET),MO)
    CFLAGS += -DTRACE_TARGET_MO
    ASMFLAGS += --defsym TRACE_TARGET=7
else ifeq ($(TRACE_TARGET),RH)
    CFLAGS += -DTRACE_TARGET_RH
    ASMFLAGS += --defsym TRACE_TARGET=2
endif

# Default EMU_PATH and OUT_PATH
EMU_PATH ?= src/emu.asm
OUT_PATH ?= build/ziskemuasm
# build/dma/memcpy_fast.o 
DMA_OBJS = build/dma/fast_dma_encode.o \
		   build/dma/direct_memcpy_mops.o build/dma/direct_memcpy_mtrace.o \
		   build/dma/direct_inputcpy_mops.o build/dma/direct_inputcpy_mtrace.o \
		   build/dma/direct_memcmp_mops.o build/dma/direct_memcmp_mtrace.o \
		   build/dma/direct_memset_mops.o build/dma/direct_memset_mtrace.o \
		   build/dma/fast_memcmp.o \
		   build/dma/fast_memcpy64.o \
		   build/dma/fast_memcpy.o \
		   build/dma/fast_memset.o \
		   build/dma/fast_inputcpy.o \
		   build/dma/memcpy_fast.o \
		   build/dma/check_dynamic_mtrace.o 

# Ensure the output directory exists
OUT_DIR := $(dir $(OUT_PATH))

# Generated constants header (included by src/constants.hpp as "generated/execution.gen.h").
# Its single canonical source is definitions/src/generated/c, produced from the #[constants]
# definitions by `cargo build -p zisk-definitions-sync`.
#   - Repo build: definitions/ is a sibling, so copy it in (and refresh when it changes).
#   - Installed/packaged build: no sibling exists, but the header was staged into
#     src/generated at package time (release.yml / build_zisk.sh), so this copy is skipped.
# Either way the compile below finds src/generated/execution.gen.h.
GEN_HEADER := src/generated/execution.gen.h
DEFS_GEN_HEADER := ../definitions/src/generated/c/execution.gen.h

$(GEN_HEADER): $(wildcard $(DEFS_GEN_HEADER))
	mkdir -p $(dir $(GEN_HEADER))
	@if [ -f "$(DEFS_GEN_HEADER)" ]; then cp -f "$(DEFS_GEN_HEADER)" "$(GEN_HEADER)"; fi
	@test -f "$(GEN_HEADER)" || { echo "ERROR: $(GEN_HEADER) missing and no definitions/ sibling to copy from (regenerate with: cargo build -p zisk-definitions-sync)"; exit 1; }

all: $(OUT_PATH)

# Compile the assembly file with a dynamic path
build/emu.o: $(EMU_PATH)
	mkdir -p build
	as $(ASMFLAGS) -o build/emu.o $<

build/dma/%.o: src/dma/%.asm src/dma/dma_constants.inc
	mkdir -p build/dma
	as $(ASMFLAGS) -I./src/dma -o $@ $<

build/dma.o: $(DMA_OBJS)
	ld -r $(DMA_OBJS) -o $@

build/zisk.ld:
	mkdir -p build
	ld --verbose 2>/dev/null | \
	    awk '/^=+$$/{found++; next} found==1{print}' | \
	    sed '/\/DISCARD\//i\  .zisk_region 0x40000000 (NOLOAD) : { . = . + 0x890000000; }' \
	    > build/zisk.ld

# Compile the final executable
$(OUT_PATH): build/zisk.ld build/emu.o $(GEN_HEADER) src/main.c src/constants.hpp src/globals.c src/globals.hpp src/configuration.c src/configuration.hpp src/trace_logs.c src/trace_logs.hpp src/server.c src/server.hpp src/client.c src/client.hpp src/c_provided.c src/c_provided.hpp src/asm_provided.hpp src/trace.c src/trace.hpp src/log.c src/log.hpp src/emu.c src/emu.hpp build/dma.o
	mkdir -p $(OUT_DIR)
	gcc $(CFLAGS)  -Wl,-T,build/zisk.ld src/main.c src/globals.c src/configuration.c src/trace_logs.c src/server.c src/client.c src/c_provided.c src/trace.c src/log.c src/emu.c -L../target/zisk-libs -L../target/release -L../target/debug -L../../bin -lc build/emu.o -lc build/dma.o -lziskc -lziskclib -lgmp -lstdc++ -lgmpxx -o $@

clean:
	rm -rf build
