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
	gcc $(CFLAGS) -c src/ffiasm/fec.cpp -o build/fecc.o
	gcc $(CFLAGS) -c src/ec/ec.cpp -o build/ec.o 
	ar rcs build/clib.a build/fec.o build/ec.o build/fecc.o
	gcc $(CFLAGS) src/main.cpp -lc build/clib.a -o build/clib -lgmp -lstdc++ -lgmpxx

clean:
	rm -rf build