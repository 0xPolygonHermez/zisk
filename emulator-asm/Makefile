# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG
      ASMFLAGS = -g
else
      CFLAGS = -O3
endif

# Default EMU_PATH and OUT_PATH
EMU_PATH ?= src/emu.asm
OUT_PATH ?= build/ziskemuasm

# Ensure the output directory exists
OUT_DIR := $(dir $(OUT_PATH))

all: $(OUT_PATH)

# Compile the assembly file with a dynamic path
build/emu.o: $(EMU_PATH)
	mkdir -p build
	as $(ASMFLAGS) -o build/emu.o $<

# Compile the final executable
$(OUT_PATH): build/emu.o src/main.c src/chfast/keccak.c
	mkdir -p $(OUT_DIR)
	gcc $(CFLAGS) -no-pie src/main.c src/chfast/keccak.c -lc build/emu.o -o $@

clean:
	rm -rf build
