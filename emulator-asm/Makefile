TRACE_TARGET=NONE

# Output and library directories. Overridable so the runtime caller
# (rom-setup) can redirect intermediates and locate libziskc.a / libziskclib.a
# without relying on the in-tree `lib-c/c/lib` and `target/{release,debug}` paths.
BUILD_DIR ?= build
ZISKC_LIB_DIR ?= ../lib-c/c/lib
ZISKCLIB_LIB_DIR ?= ../target/release

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
OUT_PATH ?= $(BUILD_DIR)/ziskemuasm
# $(BUILD_DIR)/dma/memcpy_fast.o 
DMA_OBJS = $(BUILD_DIR)/dma/fast_dma_encode.o \
		   $(BUILD_DIR)/dma/direct_memcpy_mops.o $(BUILD_DIR)/dma/direct_memcpy_mtrace.o \
		   $(BUILD_DIR)/dma/direct_inputcpy_mops.o $(BUILD_DIR)/dma/direct_inputcpy_mtrace.o \
		   $(BUILD_DIR)/dma/direct_memcmp_mops.o $(BUILD_DIR)/dma/direct_memcmp_mtrace.o \
		   $(BUILD_DIR)/dma/direct_memset_mops.o $(BUILD_DIR)/dma/direct_memset_mtrace.o \
		   $(BUILD_DIR)/dma/fast_memcmp.o \
		   $(BUILD_DIR)/dma/fast_memcpy64.o \
		   $(BUILD_DIR)/dma/fast_memcpy.o \
		   $(BUILD_DIR)/dma/fast_memset.o \
		   $(BUILD_DIR)/dma/fast_inputcpy.o \
		   $(BUILD_DIR)/dma/memcpy_fast.o \
		   $(BUILD_DIR)/dma/check_dynamic_mtrace.o 

# Ensure the output directory exists
OUT_DIR := $(dir $(OUT_PATH))

all: $(OUT_PATH)

# Compile the assembly file with a dynamic path
$(BUILD_DIR)/emu.o: $(EMU_PATH)
	mkdir -p $(BUILD_DIR)
	as $(ASMFLAGS) -o $(BUILD_DIR)/emu.o $<

DMA_INCLUDES = $(wildcard src/dma/*.inc) src/dma/fast_dma_encode_table.asm

$(BUILD_DIR)/dma/%.o: src/dma/%.asm $(DMA_INCLUDES)
	mkdir -p $(BUILD_DIR)/dma
	as $(ASMFLAGS) -I./src/dma -o $@ $<

$(BUILD_DIR)/dma.o: $(DMA_OBJS)
	ld -r $(DMA_OBJS) -o $@

$(BUILD_DIR)/zisk.ld:
	mkdir -p $(BUILD_DIR)
	ld --verbose 2>/dev/null | \
	    awk '/^=+$$/{found++; next} found==1{print}' | \
	    sed '/\/DISCARD\//i\  .zisk_region 0x40000000 (NOLOAD) : { . = . + 0x890000000; }' \
	    > $(BUILD_DIR)/zisk.ld

# Compile the final executable
$(OUT_PATH): $(BUILD_DIR)/zisk.ld $(BUILD_DIR)/emu.o src/main.c src/constants.hpp src/globals.c src/globals.hpp src/configuration.c src/configuration.hpp src/trace_logs.c src/trace_logs.hpp src/server.c src/server.hpp src/client.c src/client.hpp src/c_provided.c src/c_provided.hpp src/asm_provided.hpp src/trace.c src/trace.hpp src/log.c src/log.hpp src/emu.c src/emu.hpp $(BUILD_DIR)/dma.o
	mkdir -p $(OUT_DIR)
	gcc $(CFLAGS)  -Wl,-T,$(BUILD_DIR)/zisk.ld src/main.c src/globals.c src/configuration.c src/trace_logs.c src/server.c src/client.c src/c_provided.c src/trace.c src/log.c src/emu.c -L$(ZISKC_LIB_DIR) -L$(ZISKCLIB_LIB_DIR) -L../../bin -L../target/release -L../target/debug -lc $(BUILD_DIR)/emu.o -lc $(BUILD_DIR)/dma.o -lziskc -lziskclib -lgmp -lstdc++ -lgmpxx -o $@

clean:
	rm -rf $(BUILD_DIR)
