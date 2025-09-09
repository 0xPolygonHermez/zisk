# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG -fPIC
      ASMFLAGS = -g
else
      CFLAGS = -O3 -fPIC
endif

INC = -I ./SoftFloat-3e/source/include -I ./SoftFloat-3e/build/Linux-x86_64-GCC -I ./SoftFloat-3e/source/8086
ZISK_INC = -I ./SoftFloat-3e/source/include -I ./SoftFloat-3e/build/Linux-x86_64-GCC -I ./SoftFloat-3e/source/8086
ZISK_CFLAGS = -march=rv64ima -mabi=lp64 -ffreestanding -nostdlib -nostartfiles -mcmodel=medany
ZISK_ASM_CFLAGS = -march=rv64ima -mabi=lp64

all:
	mkdir -p build
	gcc $(CFLAGS) -c src/float/float.c $(INC) -o build/float.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_add.c $(INC) -o build/f64_add.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_addMagsF64.c $(INC) -o build/s_addMagsF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_subMagsF64.c $(INC) -o build/s_subMagsF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shiftRightJam64.c $(INC) -o build/s_shiftRightJam64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundPackToF64.c $(INC) -o build/s_roundPackToF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/softfloat_state.c $(INC) -o build/softfloat_state.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_normRoundPackToF64.c $(INC) -o build/s_normRoundPackToF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros64.c $(INC) -o build/s_countLeadingZeros64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros8.c $(INC) -o build/s_countLeadingZeros8.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/s_propagateNaNF64UI.c $(INC) -o build/s_propagateNaNF64UI.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/softfloat_raiseFlags.c $(INC) -o build/softfloat_raiseFlags.o
	ar rcs build/libziskfloat.a build/float.o build/f64_add.o build/s_addMagsF64.o build/s_subMagsF64.o build/s_shiftRightJam64.o build/s_propagateNaNF64UI.o build/s_roundPackToF64.o build/softfloat_raiseFlags.o build/softfloat_state.o build/s_normRoundPackToF64.o build/s_countLeadingZeros64.o build/s_countLeadingZeros8.o
	mkdir -p lib
	cp build/libziskfloat.a lib/
	
	gcc $(CFLAGS) src/main.cpp -lc build/libziskfloat.a -o build/ziskfloat -lgmp -lstdc++ -lgmpxx

zisk:
	mkdir -p build
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c src/float/float.c $(ZISK_INC) -o build/float.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_add.c $(ZISK_INC) -o build/f64_add.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_add.c $(ZISK_INC) -o build/f32_add.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_sub.c $(ZISK_INC) -o build/f64_sub.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_sub.c $(ZISK_INC) -o build/f32_sub.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_mul.c $(ZISK_INC) -o build/f64_mul.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_mul.c $(ZISK_INC) -o build/f32_mul.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_mulAdd.c $(ZISK_INC) -o build/f64_mulAdd.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_mulAdd.c $(ZISK_INC) -o build/f32_mulAdd.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_div.c $(ZISK_INC) -o build/f64_div.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_div.c $(ZISK_INC) -o build/f32_div.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_sqrt.c $(ZISK_INC) -o build/f64_sqrt.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_sqrt.c $(ZISK_INC) -o build/f32_sqrt.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_addM.c $(ZISK_INC) -o build/s_addM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_subM.c $(ZISK_INC) -o build/s_subM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_negXM.c $(ZISK_INC) -o build/s_negXM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_addMagsF64.c $(ZISK_INC) -o build/s_addMagsF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_addMagsF32.c $(ZISK_INC) -o build/s_addMagsF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_subMagsF64.c $(ZISK_INC) -o build/s_subMagsF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_subMagsF32.c $(ZISK_INC) -o build/s_subMagsF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_mulAddF64.c $(ZISK_INC) -o build/s_mulAddF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_mulAddF32.c $(ZISK_INC) -o build/s_mulAddF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shiftLeftM.c $(ZISK_INC) -o build/s_shiftLeftM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shortShiftLeftM.c $(ZISK_INC) -o build/s_shortShiftLeftM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightM.c $(ZISK_INC) -o build/s_shortShiftRightM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightJamM.c $(ZISK_INC) -o build/s_shortShiftRightJamM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shiftRightJam64.c $(ZISK_INC) -o build/s_shiftRightJam64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shiftRightJam32.c $(ZISK_INC) -o build/s_shiftRightJam32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shiftRightJamM.c $(ZISK_INC) -o build/s_shiftRightJamM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundPackToF64.c $(ZISK_INC) -o build/s_roundPackToF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundPackToF32.c $(ZISK_INC) -o build/s_roundPackToF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/softfloat_state.c $(ZISK_INC) -o build/softfloat_state.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_normRoundPackToF64.c $(ZISK_INC) -o build/s_normRoundPackToF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_normRoundPackToF32.c $(ZISK_INC) -o build/s_normRoundPackToF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_normSubnormalF64Sig.c $(ZISK_INC) -o build/s_normSubnormalF64Sig.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_normSubnormalF32Sig.c $(ZISK_INC) -o build/s_normSubnormalF32Sig.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros64.c $(ZISK_INC) -o build/s_countLeadingZeros64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros32.c $(ZISK_INC) -o build/s_countLeadingZeros32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros8.c $(ZISK_INC) -o build/s_countLeadingZeros8.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_mul64To128M.c $(ZISK_INC) -o build/s_mul64To128M.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_approxRecip32_1.c $(ZISK_INC) -o build/s_approxRecip32_1.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_approxRecip_1Ks.c $(ZISK_INC) -o build/s_approxRecip_1Ks.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_approxRecipSqrt32_1.c $(ZISK_INC) -o build/s_approxRecipSqrt32_1.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_approxRecipSqrt_1Ks.c $(ZISK_INC) -o build/s_approxRecipSqrt_1Ks.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightJam64.c $(ZISK_INC) -o build/s_shortShiftRightJam64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/s_propagateNaNF64UI.c $(ZISK_INC) -o build/s_propagateNaNF64UI.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/s_propagateNaNF32UI.c $(ZISK_INC) -o build/s_propagateNaNF32UI.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/softfloat_raiseFlags.c $(ZISK_INC) -o build/softfloat_raiseFlags.o
	riscv64-unknown-elf-ar rcs build/libziskfloat.a\
		build/float.o\
		build/f64_add.o\
		build/f32_add.o\
		build/f64_sub.o\
		build/f32_sub.o\
		build/f64_mul.o\
		build/f32_mul.o\
		build/f64_mulAdd.o\
		build/f32_mulAdd.o\
		build/f64_div.o\
		build/f32_div.o\
		build/f64_sqrt.o\
		build/f32_sqrt.o\
		build/s_addM.o\
		build/s_subM.o\
		build/s_negXM.o\
		build/s_addMagsF64.o\
		build/s_addMagsF32.o\
		build/s_subMagsF64.o\
		build/s_subMagsF32.o\
		build/s_mulAddF64.o\
		build/s_mulAddF32.o\
		build/s_shiftLeftM.o\
		build/s_shortShiftLeftM.o\
		build/s_shortShiftRightM.o\
		build/s_shortShiftRightJamM.o\
		build/s_shiftRightJam64.o\
		build/s_shiftRightJam32.o\
		build/s_shiftRightJamM.o\
		build/s_roundPackToF64.o\
		build/s_roundPackToF32.o\
		build/softfloat_state.o\
		build/s_normRoundPackToF64.o\
		build/s_normRoundPackToF32.o\
		build/s_normSubnormalF64Sig.o\
		build/s_normSubnormalF32Sig.o\
		build/s_countLeadingZeros64.o\
		build/s_countLeadingZeros32.o\
		build/s_countLeadingZeros8.o\
		build/s_mul64To128M.o\
		build/s_approxRecip32_1.o\
		build/s_approxRecip_1Ks.o\
		build/s_approxRecipSqrt32_1.o\
		build/s_approxRecipSqrt_1Ks.o\
		build/s_shortShiftRightJam64.o\
		build/s_propagateNaNF64UI.o\
		build/s_propagateNaNF32UI.o\
		build/softfloat_raiseFlags.o
	mkdir -p lib
	cp build/libziskfloat.a lib/

	riscv64-unknown-elf-as $(ZISK_ASM_CFLAGS) src/start.S -o build/start.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c src/main.cpp $(ZISK_INC) -o build/main.o

	riscv64-unknown-elf-ld -T scripts/link.ld build/start.o build/main.o build/libziskfloat.a -o build/ziskfloat

	# riscv64-unknown-elf-gcc $(ZISK_CFLAGS) src/main.cpp $(ZISK_INC) -o build/main.o
    # riscv64-unknown-elf-ld -T scripts/link.ld build/main.o build/libziskfloat.a -o build/ziskfloat.elf
    #riscv64-unknown-elf-ld --script scripts/link.ld build/main.o build/libziskfloat.a -o build/ziskfloat.elf

ziskdump:
	riscv64-unknown-elf-objdump -d build/libziskfloat.a > build/libziskfloat.s

clean:
	rm -rf build
	rm -rf lib