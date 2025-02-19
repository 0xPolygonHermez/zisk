all:
	mkdir -p build
	as -g -D -o build/emu.o -s src/emu.asm
	gcc -g -O3 src/main.c src/chfast/keccak.c -lc build/emu.o -lc ../target/release/libziskclib.a -o build/emu