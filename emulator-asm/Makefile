# This lock file is used to serialize builds and prevent race conditions
# when running multiple `make` commands in parallel with different EMU_PATH values.
LOCKFILE := /tmp/ziskemu_build.lock

# Debug build flags
ifeq ($(dbg),1)
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
	flock $(LOCKFILE) as $(ASMFLAGS) -o build/emu.o $<

# Compile the final executable
$(OUT_PATH): build/emu.o src/main.c src/chfast/keccak.c
	mkdir -p $(OUT_DIR)
	flock $(LOCKFILE) gcc $(CFLAGS) src/main.c src/chfast/keccak.c -lc build/emu.o -lc ../lib-c/c/lib/libziskc.a -lgmp -lstdc++ -lgmpxx -o $@

clean:
	flock $(LOCKFILE) rm -rf build
