# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG -fPIC
      ASMFLAGS = -g
else
      CFLAGS = -O3 -fPIC
endif

all:
	mkdir -p build
	nasm -felf64 src/ffiasm/fec.asm -o build/fec.o
	nasm -felf64 src/ffiasm/fnec.asm -o build/fnec.o
	nasm -felf64 src/ffiasm/fq.asm -o build/fq.o
	nasm -felf64 src/ffiasm/bls12_381_384.asm -o build/bls12_381_384.o
	gcc $(CFLAGS) -c src/ffiasm/fec.cpp -o build/fecc.o
	gcc $(CFLAGS) -c src/ffiasm/fnec.cpp -o build/fnecc.o
	gcc $(CFLAGS) -c src/ffiasm/fq.cpp -o build/fqc.o
	gcc $(CFLAGS) -c src/ffiasm/bls12_381_384.cpp -o build/bls12_381_384c.o
	gcc $(CFLAGS) -c src/ec/ec.cpp -o build/ec.o
	gcc $(CFLAGS) -c src/bn254/bn254.cpp -o build/bn254.o
	gcc $(CFLAGS) -c src/bls12_381/bls12_381.cpp -o build/bls12_381.o
	gcc $(CFLAGS) -c src/fcall/fcall.cpp -o build/fcall.o
	gcc $(CFLAGS) -c src/arith256/arith256.cpp -o build/arith256.o
	gcc $(CFLAGS) -c src/arith384/arith384.cpp -o build/arith384.o
	gcc $(CFLAGS) -c src/bigint/add256.cpp -o build/add256.o
	gcc $(CFLAGS) -c src/common/globals.cpp -o build/globals.o
	ar rcs\
		build/libziskc.a\
		build/fec.o\
		build/fnec.o\
		build/fq.o\
		build/bls12_381_384.o\
		build/ec.o\
		build/bn254.o\
		build/bls12_381.o\
		build/fecc.o\
		build/fnecc.o\
		build/fqc.o\
		build/bls12_381_384c.o\
		build/fcall.o\
		build/arith256.o\
		build/arith384.o\
		build/add256.o\
		build/globals.o
	gcc $(CFLAGS) src/main.cpp -lc build/libziskc.a -o build/clib -lgmp -lstdc++ -lgmpxx
	mkdir -p lib
	cp build/libziskc.a lib/

clean:
	rm -rf build
	rm -rf lib
