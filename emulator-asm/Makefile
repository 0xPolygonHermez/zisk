TRACE_TARGET=NONE

# Debug build flags
ifeq ($(dbg),1)
	# CFLAGS = -O0 -g -D DEBUG -no-pie -ggdb -fno-inline
	CFLAGS = -g -D DEBUG -no-pie
	ASMFLAGS = -g --noexecstack
else
	CFLAGS = -O3 -no-pie
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
	    sed '/\/DISCARD\//i\  .my_region 0x40000000 (NOLOAD) : { . = . + 0x1090000000; }' \
	    > build/zisk.ld

# Compile the final executable
$(OUT_PATH): build/zisk.ld build/emu.o src/main.c src/globals.c src/configuration.c src/trace_logs.c src/server.c src/client.c src/c_provided.c src/trace.c src/emu.c src/chfast/keccak.c src/bcon/bcon_sha256.c build/dma.o
	mkdir -p $(OUT_DIR)
	gcc $(CFLAGS)  -Wl,-T,build/zisk.ld src/main.c src/globals.c src/configuration.c src/trace_logs.c src/server.c src/client.c src/c_provided.c src/trace.c src/emu.c src/chfast/keccak.c src/bcon/bcon_sha256.c -L../lib-c/c/lib -L../../bin -L../target/release -L../target/debug -lc build/emu.o -lc build/dma.o -lziskc -lziskclib -lgmp -lstdc++ -lgmpxx -o $@

clean:
	rm -rf build
