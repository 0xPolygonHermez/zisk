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

# Main target
$(OUT_PATH): $(EMU_PATH) src/main.c src/chfast/keccak.c
	@TMPDIR=$$(mktemp -d /tmp/ziskemu_build_XXXXXX) && \
	as $(ASMFLAGS) -o $$TMPDIR/emu.o $(EMU_PATH) && \
	gcc $(CFLAGS) src/main.c src/chfast/keccak.c -lc $$TMPDIR/emu.o -lc ../lib-c/c/lib/libziskc.a -lgmp -lstdc++ -lgmpxx -o $$TMPDIR/ziskemu_bin && \
	mkdir -p $(OUT_DIR) && \
	mv $$TMPDIR/ziskemu_bin $(OUT_PATH) && \
	rm -rf $$TMPDIR

clean:
	rm -rf build
