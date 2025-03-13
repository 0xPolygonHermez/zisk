# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG -no-pie
      ASMFLAGS = -g
else
      CFLAGS = -O3 -D DEBUG -no-pie
endif

# Default EMU_PATH (can be overridden)
EMU_PATH ?= src/emu.asm

all: build/ziskemuasm

# Compile the assembly file with a dynamic path
build/emu.o: $(EMU_PATH)
	mkdir -p build
	as $(ASMFLAGS) -o build/emu.o $<

# Compile the final executable
build/ziskemuasm: build/emu.o src/main.c src/chfast/keccak.c
	gcc $(CFLAGS) -no-pie src/main.c src/chfast/keccak.c -lc build/emu.o -lc ../target/release/libziskclib.a -o build/ziskemuasm

clean:
	rm -rf build