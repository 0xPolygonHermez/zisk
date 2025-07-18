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

# Ensure the output directory exists
OUT_DIR := $(dir $(OUT_PATH))

all: $(OUT_PATH)

# Compile the assembly file with a dynamic path
build/emu.o: $(EMU_PATH)
	mkdir -p build
	as $(ASMFLAGS) -o build/emu.o $<

# Compile the final executable
$(OUT_PATH): build/emu.o src/main.c src/emu.c src/chfast/keccak.c
	mkdir -p $(OUT_DIR)
	gcc $(CFLAGS) src/main.c src/emu.c src/chfast/keccak.c src/bcon/bcon_sha256.c -L../lib-c/c/lib -L../../bin -L../target/release -L../target/debug -lc build/emu.o -lziskc -lziskclib -lgmp -lstdc++ -lgmpxx -o $@

clean:
	rm -rf build
