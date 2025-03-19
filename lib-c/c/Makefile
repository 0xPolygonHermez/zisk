# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG -fPIE
      ASMFLAGS = -g
else
      CFLAGS = -O3 -fPIE
endif

all:
	mkdir -p build
	nasm -felf64 src/ffiasm/fec.asm -o build/fec.o
	gcc $(CFLAGS) -c src/ffiasm/fec.cpp -o build/fecc.o
	gcc $(CFLAGS) -c src/ec/ec.cpp -o build/ec.o
	ar rcs build/libziskc.a build/fec.o build/ec.o build/fecc.o
	gcc $(CFLAGS) src/main.cpp -lc build/libziskc.a -o build/clib -lgmp -lstdc++ -lgmpxx
	mkdir -p lib
	cp build/libziskc.a lib/

clean:
	rm -rf build
	rm -rf lib