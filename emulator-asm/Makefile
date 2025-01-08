all:
	mkdir -p build
	as -g -D -o build/emu.o -s src/emu.asm
# ld -o emu emu.o
	gcc -g src/main.c -lc build/emu.o -o build/emu