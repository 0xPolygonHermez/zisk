# Correctness + benchmark for arith256_mod vs the legacy Rust Arith256Mod.
#
#   make        # build ./arith256_mod_bench
#   make run    # build and run
#
# Requires the Rust C-ABI lib (target/zisk-libs/libziskc.a), built by the normal zisk build.

REPO   := ../../../..
ZISKC  := $(REPO)/target/zisk-libs/libziskc.a
ASM    := ../arith256_mod.asm
LIBS   := -lstdc++ -lgmp -lgmpxx -lpthread -ldl -lm

all: arith256_mod_bench

arith256_mod.o: $(ASM)
	as --noexecstack -o $@ $<

bench_wrappers.o: bench_wrappers.asm
	as --noexecstack -o $@ $<

arith256_mod_bench: arith256_mod_bench.c arith256_mod.o bench_wrappers.o
	gcc -O2 -no-pie arith256_mod_bench.c arith256_mod.o bench_wrappers.o $(ZISKC) $(LIBS) -o $@

run: arith256_mod_bench
	./arith256_mod_bench

clean:
	rm -f arith256_mod_bench arith256_mod.o bench_wrappers.o
