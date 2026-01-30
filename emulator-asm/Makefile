# Debug build flags
ifeq ($(dbg),1)
	# CFLAGS = -O0 -g -D DEBUG -no-pie -ggdb -fno-inline
	CFLAGS = -g -D DEBUG -no-pie
	ASMFLAGS = -g --noexecstack
else
	CFLAGS = -O3 -no-pie
	ASMFLAGS = --noexecstack
endif

# Default EMU_PATH and OUT_PATH
EMU_PATH ?= src/emu.asm
OUT_PATH ?= build/ziskemuasm
DMA_OBJS = build/dma/fast_dma_encode.o build/dma/memcpy_fast.o build/dma/direct_memcpy_mtrace.o build/dma/direct_memcpy_mops.o 

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

# Compile the final executable
$(OUT_PATH): build/emu.o src/main.c src/emu.c src/chfast/keccak.c  build/dma.o
	mkdir -p $(OUT_DIR)
	gcc $(CFLAGS) src/main.c src/emu.c src/chfast/keccak.c src/bcon/bcon_sha256.c -L../lib-c/c/lib -L../../bin -L../target/release -L../target/debug -lc build/emu.o -lc build/dma.o -lziskc -lziskclib -lgmp -lstdc++ -lgmpxx -o $@

clean:
	rm -rf build
