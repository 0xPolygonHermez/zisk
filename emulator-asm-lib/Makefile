ASMFLAGS = -msyntax=intel -mintel64 -L
CFLAGS = -fPIC
# Debug build flags
ifeq ($(dbg),1)
      CFLAGS += -g -D DEBUG
      ASMFLAGS += -g #--64 #--noexecstack
else
      CFLAGS = -O3
#	  ASMFLAGS = --64
#	  ASMFLAGS = --noexecstack
endif

# Default EMU_PATH and OUT_PATH
EMU_PATH ?= src/emu.asm
OUT_PATH ?= build/ziskemuasm.so

# Ensure the output directory exists
OUT_DIR := $(dir $(OUT_PATH))

all: $(OUT_PATH)

# Compile the assembly file with a dynamic path
build/emu.o: $(EMU_PATH)
	mkdir -p build
#	gcc -c -fPIC src/emu.asm -masm=intel -o build/emu.o
#	nasm -f as86 src/emu.asm -o src/emu.S
#	gcc -c src/emu.S -o src/emu.o
#	cp src/emu.s src/emu.S
#	gcc -E src/emu.S -o src/emu.asm 
#	nasm $(ASMFLAGS) -f elf64 -O0 -o build/emu.o src/emu.asm
#	nasm $(ASMFLAGS) -f elf64 -E -w+gnu-elf-extensions -o build/emu.o src/emu.asm
	as $(ASMFLAGS) -o build/emu.o $<

# Compile the final executable
$(OUT_PATH): build/emu.o src/lib.c ../emulator-asm/src/emu.c ../emulator-asm/src/chfast/keccak.c
	mkdir -p $(OUT_DIR)
#	gcc $(CFLAGS) src/lib.c ../emulator-asm/src/emu.c ../emulator-asm/src/chfast/keccak.c -lc build/emu.o -lc ../lib-c/c/lib/libziskc.a -lgmp -lstdc++ -lgmpxx -shared -o $@
	gcc $(CFLAGS) -m64 src/lib.c ../emulator-asm/src/emu.c ../emulator-asm/src/chfast/keccak.c -lc build/emu.o -lgmp -lstdc++ -lgmpxx -shared -o $@

clean:
	rm -rf build
