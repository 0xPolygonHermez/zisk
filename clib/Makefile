# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG
      ASMFLAGS = -g
else
      CFLAGS = -O3
endif

all:
	mkdir -p build
	nasm -felf64 src/ffiasm/fec.asm -o build/fec.o
	gcc $(CFLAGS) src/main.cpp src/ffiasm/fec.cpp src/ec/ec.cpp -lc build/fec.o -lgmp -lstdc++ -lgmpxx -o build/clib


clean:
	rm -rf build