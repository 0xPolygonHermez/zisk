# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG -D ZISK_GCC -fPIC
      ASMFLAGS = -g
else
      CFLAGS = -O3 -D ZISK_GCC -fPIC
endif

INC = -I ./SoftFloat-3e/source/include -I ./SoftFloat-3e/build/Linux-x86_64-GCC -I ./SoftFloat-3e/source/8086
ZISK_INC = -I ./SoftFloat-3e/source/include -I ./SoftFloat-3e/build/Linux-x86_64-GCC -I ./SoftFloat-3e/source/8086
# `-fno-ident` suppresses the `.comment` section gcc would otherwise embed
# (which holds the compiler version string and breaks byte-determinism across
# different gcc versions — necessary so the vendored fallback at
# `lib-float/c/lib/ziskfloat.elf` matches a fresh build on any contributor's
# machine).
ZISK_CFLAGS = -march=rv64ima -mabi=lp64 -ffreestanding -nostdlib -nostartfiles -mcmodel=medany -fno-ident
ZISK_ASM_CFLAGS = -march=rv64ima -mabi=lp64

# Output directories (overridable so the Rust build script can redirect into OUT_DIR)
BUILD_DIR ?= build
LIB_DIR ?= lib

all:
	mkdir -p $(BUILD_DIR)

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c src/float/float.c $(ZISK_INC) -o $(BUILD_DIR)/float.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_add.c $(ZISK_INC) -o $(BUILD_DIR)/f64_add.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_add.c $(ZISK_INC) -o $(BUILD_DIR)/f32_add.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_sub.c $(ZISK_INC) -o $(BUILD_DIR)/f64_sub.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_sub.c $(ZISK_INC) -o $(BUILD_DIR)/f32_sub.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_mul.c $(ZISK_INC) -o $(BUILD_DIR)/f64_mul.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_mul.c $(ZISK_INC) -o $(BUILD_DIR)/f32_mul.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_mulAdd.c $(ZISK_INC) -o $(BUILD_DIR)/f64_mulAdd.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_mulAdd.c $(ZISK_INC) -o $(BUILD_DIR)/f32_mulAdd.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_div.c $(ZISK_INC) -o $(BUILD_DIR)/f64_div.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_div.c $(ZISK_INC) -o $(BUILD_DIR)/f32_div.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_sqrt.c $(ZISK_INC) -o $(BUILD_DIR)/f64_sqrt.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_sqrt.c $(ZISK_INC) -o $(BUILD_DIR)/f32_sqrt.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_eq.c $(ZISK_INC) -o $(BUILD_DIR)/f64_eq.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_eq.c $(ZISK_INC) -o $(BUILD_DIR)/f32_eq.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_lt.c $(ZISK_INC) -o $(BUILD_DIR)/f64_lt.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_lt.c $(ZISK_INC) -o $(BUILD_DIR)/f32_lt.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_le.c $(ZISK_INC) -o $(BUILD_DIR)/f64_le.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_le.c $(ZISK_INC) -o $(BUILD_DIR)/f32_le.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_addM.c $(ZISK_INC) -o $(BUILD_DIR)/s_addM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_subM.c $(ZISK_INC) -o $(BUILD_DIR)/s_subM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_negXM.c $(ZISK_INC) -o $(BUILD_DIR)/s_negXM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_addMagsF64.c $(ZISK_INC) -o $(BUILD_DIR)/s_addMagsF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_addMagsF32.c $(ZISK_INC) -o $(BUILD_DIR)/s_addMagsF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_subMagsF64.c $(ZISK_INC) -o $(BUILD_DIR)/s_subMagsF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_subMagsF32.c $(ZISK_INC) -o $(BUILD_DIR)/s_subMagsF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_mulAddF64.c $(ZISK_INC) -o $(BUILD_DIR)/s_mulAddF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_mulAddF32.c $(ZISK_INC) -o $(BUILD_DIR)/s_mulAddF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shiftLeftM.c $(ZISK_INC) -o $(BUILD_DIR)/s_shiftLeftM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shortShiftLeftM.c $(ZISK_INC) -o $(BUILD_DIR)/s_shortShiftLeftM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightM.c $(ZISK_INC) -o $(BUILD_DIR)/s_shortShiftRightM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightJamM.c $(ZISK_INC) -o $(BUILD_DIR)/s_shortShiftRightJamM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shiftRightJam64.c $(ZISK_INC) -o $(BUILD_DIR)/s_shiftRightJam64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shiftRightJam32.c $(ZISK_INC) -o $(BUILD_DIR)/s_shiftRightJam32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shiftRightJamM.c $(ZISK_INC) -o $(BUILD_DIR)/s_shiftRightJamM.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundPackToF64.c $(ZISK_INC) -o $(BUILD_DIR)/s_roundPackToF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundPackToF32.c $(ZISK_INC) -o $(BUILD_DIR)/s_roundPackToF32.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundToUI64.c $(ZISK_INC) -o $(BUILD_DIR)/s_roundMoUI64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundToI64.c $(ZISK_INC) -o $(BUILD_DIR)/s_roundMoI64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundToUI32.c $(ZISK_INC) -o $(BUILD_DIR)/s_roundMoUI32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundToI32.c $(ZISK_INC) -o $(BUILD_DIR)/s_roundMoI32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundMToUI64.c $(ZISK_INC) -o $(BUILD_DIR)/s_roundMToUI64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_roundMToI64.c $(ZISK_INC) -o $(BUILD_DIR)/s_roundMToI64.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/softfloat_state.c $(ZISK_INC) -o $(BUILD_DIR)/softfloat_state.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_normRoundPackToF64.c $(ZISK_INC) -o $(BUILD_DIR)/s_normRoundPackToF64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_normRoundPackToF32.c $(ZISK_INC) -o $(BUILD_DIR)/s_normRoundPackToF32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_normSubnormalF64Sig.c $(ZISK_INC) -o $(BUILD_DIR)/s_normSubnormalF64Sig.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_normSubnormalF32Sig.c $(ZISK_INC) -o $(BUILD_DIR)/s_normSubnormalF32Sig.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros64.c $(ZISK_INC) -o $(BUILD_DIR)/s_countLeadingZeros64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros32.c $(ZISK_INC) -o $(BUILD_DIR)/s_countLeadingZeros32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros8.c $(ZISK_INC) -o $(BUILD_DIR)/s_countLeadingZeros8.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_mul64To128M.c $(ZISK_INC) -o $(BUILD_DIR)/s_mul64To128M.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_approxRecip32_1.c $(ZISK_INC) -o $(BUILD_DIR)/s_approxRecip32_1.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_approxRecip_1Ks.c $(ZISK_INC) -o $(BUILD_DIR)/s_approxRecip_1Ks.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_approxRecipSqrt32_1.c $(ZISK_INC) -o $(BUILD_DIR)/s_approxRecipSqrt32_1.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_approxRecipSqrt_1Ks.c $(ZISK_INC) -o $(BUILD_DIR)/s_approxRecipSqrt_1Ks.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightJam64.c $(ZISK_INC) -o $(BUILD_DIR)/s_shortShiftRightJam64.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/s_propagateNaNF64UI.c $(ZISK_INC) -o $(BUILD_DIR)/s_propagateNaNF64UI.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/s_propagateNaNF32UI.c $(ZISK_INC) -o $(BUILD_DIR)/s_propagateNaNF32UI.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/softfloat_raiseFlags.c $(ZISK_INC) -o $(BUILD_DIR)/softfloat_raiseFlags.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/s_commonNaNToF64UI.c $(ZISK_INC) -o $(BUILD_DIR)/s_commonNaNToF64UI.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/s_commonNaNToF32UI.c $(ZISK_INC) -o $(BUILD_DIR)/s_commonNaNToF32UI.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/s_f64UIToCommonNaN.c $(ZISK_INC) -o $(BUILD_DIR)/s_f64UIToCommonNaN.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/8086/s_f32UIToCommonNaN.c $(ZISK_INC) -o $(BUILD_DIR)/s_f32UIToCommonNaN.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_to_i32.c $(ZISK_INC) -o $(BUILD_DIR)/f32_to_i32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_to_ui32.c $(ZISK_INC) -o $(BUILD_DIR)/f32_to_ui32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_to_i64.c $(ZISK_INC) -o $(BUILD_DIR)/f32_to_i64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_to_ui64.c $(ZISK_INC) -o $(BUILD_DIR)/f32_to_ui64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_to_i32.c $(ZISK_INC) -o $(BUILD_DIR)/f64_to_i32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_to_ui32.c $(ZISK_INC) -o $(BUILD_DIR)/f64_to_ui32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_to_i64.c $(ZISK_INC) -o $(BUILD_DIR)/f64_to_i64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_to_ui64.c $(ZISK_INC) -o $(BUILD_DIR)/f64_to_ui64.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/i32_to_f32.c $(ZISK_INC) -o $(BUILD_DIR)/i32_to_f32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/ui32_to_f32.c $(ZISK_INC) -o $(BUILD_DIR)/ui32_to_f32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/i64_to_f32.c $(ZISK_INC) -o $(BUILD_DIR)/i64_to_f32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/ui64_to_f32.c $(ZISK_INC) -o $(BUILD_DIR)/ui64_to_f32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/i32_to_f64.c $(ZISK_INC) -o $(BUILD_DIR)/i32_to_f64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/ui32_to_f64.c $(ZISK_INC) -o $(BUILD_DIR)/ui32_to_f64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/i64_to_f64.c $(ZISK_INC) -o $(BUILD_DIR)/i64_to_f64.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/ui64_to_f64.c $(ZISK_INC) -o $(BUILD_DIR)/ui64_to_f64.o

	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f64_to_f32.c $(ZISK_INC) -o $(BUILD_DIR)/f64_to_f32.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c SoftFloat-3e/source/f32_to_f64.c $(ZISK_INC) -o $(BUILD_DIR)/f32_to_f64.o

	riscv64-unknown-elf-ar Drcs $(BUILD_DIR)/libziskfloat.a\
		$(BUILD_DIR)/float.o\
		$(BUILD_DIR)/f64_add.o\
		$(BUILD_DIR)/f32_add.o\
		$(BUILD_DIR)/f64_sub.o\
		$(BUILD_DIR)/f32_sub.o\
		$(BUILD_DIR)/f64_mul.o\
		$(BUILD_DIR)/f32_mul.o\
		$(BUILD_DIR)/f64_mulAdd.o\
		$(BUILD_DIR)/f32_mulAdd.o\
		$(BUILD_DIR)/f64_div.o\
		$(BUILD_DIR)/f32_div.o\
		$(BUILD_DIR)/f64_sqrt.o\
		$(BUILD_DIR)/f32_sqrt.o\
		$(BUILD_DIR)/f64_eq.o\
		$(BUILD_DIR)/f32_eq.o\
		$(BUILD_DIR)/f64_lt.o\
		$(BUILD_DIR)/f32_lt.o\
		$(BUILD_DIR)/f64_le.o\
		$(BUILD_DIR)/f32_le.o\
		$(BUILD_DIR)/s_addM.o\
		$(BUILD_DIR)/s_subM.o\
		$(BUILD_DIR)/s_negXM.o\
		$(BUILD_DIR)/s_addMagsF64.o\
		$(BUILD_DIR)/s_addMagsF32.o\
		$(BUILD_DIR)/s_subMagsF64.o\
		$(BUILD_DIR)/s_subMagsF32.o\
		$(BUILD_DIR)/s_mulAddF64.o\
		$(BUILD_DIR)/s_mulAddF32.o\
		$(BUILD_DIR)/s_shiftLeftM.o\
		$(BUILD_DIR)/s_shortShiftLeftM.o\
		$(BUILD_DIR)/s_shortShiftRightM.o\
		$(BUILD_DIR)/s_shortShiftRightJamM.o\
		$(BUILD_DIR)/s_shiftRightJam64.o\
		$(BUILD_DIR)/s_shiftRightJam32.o\
		$(BUILD_DIR)/s_shiftRightJamM.o\
		$(BUILD_DIR)/s_roundPackToF64.o\
		$(BUILD_DIR)/s_roundPackToF32.o\
	 	$(BUILD_DIR)/s_roundMoUI64.o\
		$(BUILD_DIR)/s_roundMoI64.o\
	 	$(BUILD_DIR)/s_roundMoUI32.o\
		$(BUILD_DIR)/s_roundMoI32.o\
		$(BUILD_DIR)/s_roundMToUI64.o\
	 	$(BUILD_DIR)/s_roundMToI64.o\
		$(BUILD_DIR)/softfloat_state.o\
		$(BUILD_DIR)/s_normRoundPackToF64.o\
		$(BUILD_DIR)/s_normRoundPackToF32.o\
		$(BUILD_DIR)/s_normSubnormalF64Sig.o\
		$(BUILD_DIR)/s_normSubnormalF32Sig.o\
		$(BUILD_DIR)/s_countLeadingZeros64.o\
		$(BUILD_DIR)/s_countLeadingZeros32.o\
		$(BUILD_DIR)/s_countLeadingZeros8.o\
		$(BUILD_DIR)/s_mul64To128M.o\
		$(BUILD_DIR)/s_approxRecip32_1.o\
		$(BUILD_DIR)/s_approxRecip_1Ks.o\
		$(BUILD_DIR)/s_approxRecipSqrt32_1.o\
		$(BUILD_DIR)/s_approxRecipSqrt_1Ks.o\
		$(BUILD_DIR)/s_shortShiftRightJam64.o\
		$(BUILD_DIR)/s_propagateNaNF64UI.o\
		$(BUILD_DIR)/s_propagateNaNF32UI.o\
		$(BUILD_DIR)/softfloat_raiseFlags.o\
		$(BUILD_DIR)/s_commonNaNToF64UI.o\
		$(BUILD_DIR)/s_commonNaNToF32UI.o\
		$(BUILD_DIR)/s_f64UIToCommonNaN.o\
		$(BUILD_DIR)/s_f32UIToCommonNaN.o\
		$(BUILD_DIR)/f32_to_i32.o\
		$(BUILD_DIR)/f32_to_ui32.o\
		$(BUILD_DIR)/f32_to_i64.o\
		$(BUILD_DIR)/f32_to_ui64.o\
		$(BUILD_DIR)/f64_to_i32.o\
		$(BUILD_DIR)/f64_to_ui32.o\
		$(BUILD_DIR)/f64_to_i64.o\
		$(BUILD_DIR)/f64_to_ui64.o\
		$(BUILD_DIR)/i32_to_f32.o\
		$(BUILD_DIR)/ui32_to_f32.o\
		$(BUILD_DIR)/i64_to_f32.o\
		$(BUILD_DIR)/ui64_to_f32.o\
		$(BUILD_DIR)/i32_to_f64.o\
		$(BUILD_DIR)/ui32_to_f64.o\
		$(BUILD_DIR)/i64_to_f64.o\
		$(BUILD_DIR)/ui64_to_f64.o\
		$(BUILD_DIR)/f64_to_f32.o\
		$(BUILD_DIR)/f32_to_f64.o

	mkdir -p $(LIB_DIR)
	cp $(BUILD_DIR)/libziskfloat.a $(LIB_DIR)/

	riscv64-unknown-elf-as $(ZISK_ASM_CFLAGS) src/start.S -o $(BUILD_DIR)/start.o
	riscv64-unknown-elf-gcc $(ZISK_CFLAGS) -c src/main.cpp $(ZISK_INC) -o $(BUILD_DIR)/main.o
	# `--build-id=none` forces no build-id note regardless of binutils default
	# (some distros enable build-id by default, which would inject a random
	# hash on every link and break the deterministic vendored-fallback contract).
	riscv64-unknown-elf-ld --build-id=none -T scripts/link_main.ld $(BUILD_DIR)/start.o $(BUILD_DIR)/main.o $(BUILD_DIR)/libziskfloat.a -o $(BUILD_DIR)/ziskfloatmain.elf
	riscv64-unknown-elf-ld --build-id=none -T scripts/link.ld $(BUILD_DIR)/libziskfloat.a -o $(BUILD_DIR)/ziskfloat.elf
	cp $(BUILD_DIR)/ziskfloat.elf $(LIB_DIR)/

ziskdump:
	riscv64-unknown-elf-objdump -d $(BUILD_DIR)/libziskfloat.a > $(BUILD_DIR)/libziskfloat.s

clean:
	rm -rf $(BUILD_DIR)
	rm -rf $(LIB_DIR)

zisk_gcc:
	mkdir -p $(BUILD_DIR)

	gcc $(CFLAGS) -c src/float/float.c $(INC) -o $(BUILD_DIR)/float.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_add.c $(INC) -o $(BUILD_DIR)/f64_add.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_add.c $(INC) -o $(BUILD_DIR)/f32_add.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_sub.c $(INC) -o $(BUILD_DIR)/f64_sub.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_sub.c $(INC) -o $(BUILD_DIR)/f32_sub.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_mul.c $(INC) -o $(BUILD_DIR)/f64_mul.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_mul.c $(INC) -o $(BUILD_DIR)/f32_mul.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_mulAdd.c $(INC) -o $(BUILD_DIR)/f64_mulAdd.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_mulAdd.c $(INC) -o $(BUILD_DIR)/f32_mulAdd.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_div.c $(INC) -o $(BUILD_DIR)/f64_div.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_div.c $(INC) -o $(BUILD_DIR)/f32_div.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_sqrt.c $(INC) -o $(BUILD_DIR)/f64_sqrt.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_sqrt.c $(INC) -o $(BUILD_DIR)/f32_sqrt.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_eq.c $(INC) -o $(BUILD_DIR)/f64_eq.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_eq.c $(INC) -o $(BUILD_DIR)/f32_eq.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_lt.c $(INC) -o $(BUILD_DIR)/f64_lt.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_lt.c $(INC) -o $(BUILD_DIR)/f32_lt.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_le.c $(INC) -o $(BUILD_DIR)/f64_le.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_le.c $(INC) -o $(BUILD_DIR)/f32_le.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/s_addM.c $(INC) -o $(BUILD_DIR)/s_addM.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_subM.c $(INC) -o $(BUILD_DIR)/s_subM.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_negXM.c $(INC) -o $(BUILD_DIR)/s_negXM.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_addMagsF64.c $(INC) -o $(BUILD_DIR)/s_addMagsF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_addMagsF32.c $(INC) -o $(BUILD_DIR)/s_addMagsF32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_subMagsF64.c $(INC) -o $(BUILD_DIR)/s_subMagsF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_subMagsF32.c $(INC) -o $(BUILD_DIR)/s_subMagsF32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_mulAddF64.c $(INC) -o $(BUILD_DIR)/s_mulAddF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_mulAddF32.c $(INC) -o $(BUILD_DIR)/s_mulAddF32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shiftLeftM.c $(INC) -o $(BUILD_DIR)/s_shiftLeftM.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shortShiftLeftM.c $(INC) -o $(BUILD_DIR)/s_shortShiftLeftM.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightM.c $(INC) -o $(BUILD_DIR)/s_shortShiftRightM.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightJamM.c $(INC) -o $(BUILD_DIR)/s_shortShiftRightJamM.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shiftRightJam64.c $(INC) -o $(BUILD_DIR)/s_shiftRightJam64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shiftRightJam32.c $(INC) -o $(BUILD_DIR)/s_shiftRightJam32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shiftRightJamM.c $(INC) -o $(BUILD_DIR)/s_shiftRightJamM.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundPackToF64.c $(INC) -o $(BUILD_DIR)/s_roundPackToF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundPackToF32.c $(INC) -o $(BUILD_DIR)/s_roundPackToF32.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundToUI64.c $(INC) -o $(BUILD_DIR)/s_roundMoUI64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundToI64.c $(INC) -o $(BUILD_DIR)/s_roundMoI64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundToUI32.c $(INC) -o $(BUILD_DIR)/s_roundMoUI32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundToI32.c $(INC) -o $(BUILD_DIR)/s_roundMoI32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundMToUI64.c $(INC) -o $(BUILD_DIR)/s_roundMToUI64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_roundMToI64.c $(INC) -o $(BUILD_DIR)/s_roundMToI64.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/softfloat_state.c $(INC) -o $(BUILD_DIR)/softfloat_state.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_normRoundPackToF64.c $(INC) -o $(BUILD_DIR)/s_normRoundPackToF64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_normRoundPackToF32.c $(INC) -o $(BUILD_DIR)/s_normRoundPackToF32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_normSubnormalF64Sig.c $(INC) -o $(BUILD_DIR)/s_normSubnormalF64Sig.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_normSubnormalF32Sig.c $(INC) -o $(BUILD_DIR)/s_normSubnormalF32Sig.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros64.c $(INC) -o $(BUILD_DIR)/s_countLeadingZeros64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros32.c $(INC) -o $(BUILD_DIR)/s_countLeadingZeros32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_countLeadingZeros8.c $(INC) -o $(BUILD_DIR)/s_countLeadingZeros8.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_mul64To128M.c $(INC) -o $(BUILD_DIR)/s_mul64To128M.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_approxRecip32_1.c $(INC) -o $(BUILD_DIR)/s_approxRecip32_1.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_approxRecip_1Ks.c $(INC) -o $(BUILD_DIR)/s_approxRecip_1Ks.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_approxRecipSqrt32_1.c $(INC) -o $(BUILD_DIR)/s_approxRecipSqrt32_1.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_approxRecipSqrt_1Ks.c $(INC) -o $(BUILD_DIR)/s_approxRecipSqrt_1Ks.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/s_shortShiftRightJam64.c $(INC) -o $(BUILD_DIR)/s_shortShiftRightJam64.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/s_propagateNaNF64UI.c $(INC) -o $(BUILD_DIR)/s_propagateNaNF64UI.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/s_propagateNaNF32UI.c $(INC) -o $(BUILD_DIR)/s_propagateNaNF32UI.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/softfloat_raiseFlags.c $(INC) -o $(BUILD_DIR)/softfloat_raiseFlags.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/s_commonNaNToF64UI.c $(INC) -o $(BUILD_DIR)/s_commonNaNToF64UI.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/s_commonNaNToF32UI.c $(INC) -o $(BUILD_DIR)/s_commonNaNToF32UI.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/s_f64UIToCommonNaN.c $(INC) -o $(BUILD_DIR)/s_f64UIToCommonNaN.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/8086/s_f32UIToCommonNaN.c $(INC) -o $(BUILD_DIR)/s_f32UIToCommonNaN.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_to_i32.c $(INC) -o $(BUILD_DIR)/f32_to_i32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_to_ui32.c $(INC) -o $(BUILD_DIR)/f32_to_ui32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_to_i64.c $(INC) -o $(BUILD_DIR)/f32_to_i64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_to_ui64.c $(INC) -o $(BUILD_DIR)/f32_to_ui64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_to_i32.c $(INC) -o $(BUILD_DIR)/f64_to_i32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_to_ui32.c $(INC) -o $(BUILD_DIR)/f64_to_ui32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_to_i64.c $(INC) -o $(BUILD_DIR)/f64_to_i64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_to_ui64.c $(INC) -o $(BUILD_DIR)/f64_to_ui64.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/i32_to_f32.c $(INC) -o $(BUILD_DIR)/i32_to_f32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/ui32_to_f32.c $(INC) -o $(BUILD_DIR)/ui32_to_f32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/i64_to_f32.c $(INC) -o $(BUILD_DIR)/i64_to_f32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/ui64_to_f32.c $(INC) -o $(BUILD_DIR)/ui64_to_f32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/i32_to_f64.c $(INC) -o $(BUILD_DIR)/i32_to_f64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/ui32_to_f64.c $(INC) -o $(BUILD_DIR)/ui32_to_f64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/i64_to_f64.c $(INC) -o $(BUILD_DIR)/i64_to_f64.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/ui64_to_f64.c $(INC) -o $(BUILD_DIR)/ui64_to_f64.o

	gcc $(CFLAGS) -c SoftFloat-3e/source/f64_to_f32.c $(INC) -o $(BUILD_DIR)/f64_to_f32.o
	gcc $(CFLAGS) -c SoftFloat-3e/source/f32_to_f64.c $(INC) -o $(BUILD_DIR)/f32_to_f64.o

	ar Drcs $(BUILD_DIR)/libziskfloat.a\
		$(BUILD_DIR)/float.o\
		$(BUILD_DIR)/f64_add.o\
		$(BUILD_DIR)/f32_add.o\
		$(BUILD_DIR)/f64_sub.o\
		$(BUILD_DIR)/f32_sub.o\
		$(BUILD_DIR)/f64_mul.o\
		$(BUILD_DIR)/f32_mul.o\
		$(BUILD_DIR)/f64_mulAdd.o\
		$(BUILD_DIR)/f32_mulAdd.o\
		$(BUILD_DIR)/f64_div.o\
		$(BUILD_DIR)/f32_div.o\
		$(BUILD_DIR)/f64_sqrt.o\
		$(BUILD_DIR)/f32_sqrt.o\
		$(BUILD_DIR)/f64_eq.o\
		$(BUILD_DIR)/f32_eq.o\
		$(BUILD_DIR)/f64_lt.o\
		$(BUILD_DIR)/f32_lt.o\
		$(BUILD_DIR)/f64_le.o\
		$(BUILD_DIR)/f32_le.o\
		$(BUILD_DIR)/s_addM.o\
		$(BUILD_DIR)/s_subM.o\
		$(BUILD_DIR)/s_negXM.o\
		$(BUILD_DIR)/s_addMagsF64.o\
		$(BUILD_DIR)/s_addMagsF32.o\
		$(BUILD_DIR)/s_subMagsF64.o\
		$(BUILD_DIR)/s_subMagsF32.o\
		$(BUILD_DIR)/s_mulAddF64.o\
		$(BUILD_DIR)/s_mulAddF32.o\
		$(BUILD_DIR)/s_shiftLeftM.o\
		$(BUILD_DIR)/s_shortShiftLeftM.o\
		$(BUILD_DIR)/s_shortShiftRightM.o\
		$(BUILD_DIR)/s_shortShiftRightJamM.o\
		$(BUILD_DIR)/s_shiftRightJam64.o\
		$(BUILD_DIR)/s_shiftRightJam32.o\
		$(BUILD_DIR)/s_shiftRightJamM.o\
		$(BUILD_DIR)/s_roundPackToF64.o\
		$(BUILD_DIR)/s_roundPackToF32.o\
	 	$(BUILD_DIR)/s_roundMoUI64.o\
		$(BUILD_DIR)/s_roundMoI64.o\
	 	$(BUILD_DIR)/s_roundMoUI32.o\
		$(BUILD_DIR)/s_roundMoI32.o\
		$(BUILD_DIR)/s_roundMToUI64.o\
	 	$(BUILD_DIR)/s_roundMToI64.o\
		$(BUILD_DIR)/softfloat_state.o\
		$(BUILD_DIR)/s_normRoundPackToF64.o\
		$(BUILD_DIR)/s_normRoundPackToF32.o\
		$(BUILD_DIR)/s_normSubnormalF64Sig.o\
		$(BUILD_DIR)/s_normSubnormalF32Sig.o\
		$(BUILD_DIR)/s_countLeadingZeros64.o\
		$(BUILD_DIR)/s_countLeadingZeros32.o\
		$(BUILD_DIR)/s_countLeadingZeros8.o\
		$(BUILD_DIR)/s_mul64To128M.o\
		$(BUILD_DIR)/s_approxRecip32_1.o\
		$(BUILD_DIR)/s_approxRecip_1Ks.o\
		$(BUILD_DIR)/s_approxRecipSqrt32_1.o\
		$(BUILD_DIR)/s_approxRecipSqrt_1Ks.o\
		$(BUILD_DIR)/s_shortShiftRightJam64.o\
		$(BUILD_DIR)/s_propagateNaNF64UI.o\
		$(BUILD_DIR)/s_propagateNaNF32UI.o\
		$(BUILD_DIR)/softfloat_raiseFlags.o\
		$(BUILD_DIR)/s_commonNaNToF64UI.o\
		$(BUILD_DIR)/s_commonNaNToF32UI.o\
		$(BUILD_DIR)/s_f64UIToCommonNaN.o\
		$(BUILD_DIR)/s_f32UIToCommonNaN.o\
		$(BUILD_DIR)/f32_to_i32.o\
		$(BUILD_DIR)/f32_to_ui32.o\
		$(BUILD_DIR)/f32_to_i64.o\
		$(BUILD_DIR)/f32_to_ui64.o\
		$(BUILD_DIR)/f64_to_i32.o\
		$(BUILD_DIR)/f64_to_ui32.o\
		$(BUILD_DIR)/f64_to_i64.o\
		$(BUILD_DIR)/f64_to_ui64.o\
		$(BUILD_DIR)/i32_to_f32.o\
		$(BUILD_DIR)/ui32_to_f32.o\
		$(BUILD_DIR)/i64_to_f32.o\
		$(BUILD_DIR)/ui64_to_f32.o\
		$(BUILD_DIR)/i32_to_f64.o\
		$(BUILD_DIR)/ui32_to_f64.o\
		$(BUILD_DIR)/i64_to_f64.o\
		$(BUILD_DIR)/ui64_to_f64.o\
		$(BUILD_DIR)/f64_to_f32.o\
		$(BUILD_DIR)/f32_to_f64.o

	mkdir -p $(LIB_DIR)
	cp $(BUILD_DIR)/libziskfloat.a $(LIB_DIR)/
	
	gcc $(CFLAGS) src/main.cpp -lc $(BUILD_DIR)/libziskfloat.a -o $(BUILD_DIR)/ziskfloat -lgmp -lstdc++ -lgmpxx