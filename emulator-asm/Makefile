# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG
      ASMFLAGS = -g
else
      CFLAGS = -O3
endif

all:
	mkdir -p build
	as $(ASMFLAGS) -D -o build/emu.o -s src/emu.asm
	gcc $(CFLAGS) src/main.c src/chfast/keccak.c -lc build/emu.o -lc ../target/release/libziskclib.a -o build/emu