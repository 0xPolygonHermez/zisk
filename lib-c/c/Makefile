# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG -fPIC
      ASMFLAGS = -g
else
      CFLAGS = -O3 -fPIC
endif

# Platform / arch detection
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

# Field backend selection:
#   asm_x86_64  -> .asm via NASM (Linux x86_64) + .cpp wrapper
#   asm_arm64   -> raw_arm64.s (GAS) + _generic.cpp (high-level) + .cpp wrapper
#   generic     -> _raw_generic.cpp + _generic.cpp + .cpp wrapper (pure C, GMP mpn_*)
ifeq ($(USE_ASM),0)
    FIELD_BACKEND := generic
else ifeq ($(UNAME_S),Linux)
    ifeq ($(UNAME_M),x86_64)
        FIELD_BACKEND := asm_x86_64
    else ifneq (,$(filter aarch64 arm64,$(UNAME_M)))
        FIELD_BACKEND := asm_arm64
    else
        FIELD_BACKEND := generic
    endif
else ifeq ($(UNAME_S),Darwin)
    # macOS: NASM -felf64 won't link in a Mach-O archive; use pure-C fallback.
    FIELD_BACKEND := generic
else
    FIELD_BACKEND := generic
endif

ifeq ($(FIELD_BACKEND),asm_x86_64)
    CFLAGS += -DUSE_ASM -DARCH_X86_64
else ifeq ($(FIELD_BACKEND),asm_arm64)
    CFLAGS += -DUSE_ASM -DARCH_ARM64
endif

# When the system GMP uses `unsigned long` for mp_limb_t and our code uses
# uint64_t (== unsigned long long), C++ rejects the pointer mismatch even
# though both are 64-bit. Force GMP to typedef mp_limb_t as unsigned long
# long so the types match. Safe on any LP64 platform with 64-bit GMP limbs.
ifeq ($(UNAME_S),Darwin)
    CFLAGS += -D_LONG_LONG_LIMB
endif

# Directories
BUILD_DIR = build
LIB_DIR = lib
SRC_DIR = src

# Output library
LIB_NAME = libziskc.a
LIB_FILE = $(LIB_DIR)/$(LIB_NAME)

# Field list
FIELDS = fr fq fec fnec bls12_381 bls12_381_384 nsecp256r1 psecp256r1

# Per-field object selection
ifeq ($(FIELD_BACKEND),asm_x86_64)
    FIELD_ASM_OBJS = $(addprefix $(BUILD_DIR)/,$(addsuffix _asm.o,$(FIELDS)))
    FIELD_CPP_OBJS = $(addprefix $(BUILD_DIR)/,$(addsuffix _c.o,$(FIELDS)))
else ifeq ($(FIELD_BACKEND),asm_arm64)
    FIELD_ASM_OBJS = $(addprefix $(BUILD_DIR)/,$(addsuffix _raw_arm64.o,$(FIELDS)))
    FIELD_CPP_OBJS = $(addprefix $(BUILD_DIR)/,$(addsuffix _c.o,$(FIELDS))) \
                     $(addprefix $(BUILD_DIR)/,$(addsuffix _generic.o,$(FIELDS)))
else
    FIELD_ASM_OBJS =
    FIELD_CPP_OBJS = $(addprefix $(BUILD_DIR)/,$(addsuffix _c.o,$(FIELDS))) \
                     $(addprefix $(BUILD_DIR)/,$(addsuffix _generic.o,$(FIELDS))) \
                     $(addprefix $(BUILD_DIR)/,$(addsuffix _raw_generic.o,$(FIELDS)))
endif

# Non-field object files (always built)
OTHER_OBJS = $(BUILD_DIR)/alt_bn128.o \
             $(BUILD_DIR)/secp256r1.o \
             $(BUILD_DIR)/misc.o \
             $(BUILD_DIR)/naf.o \
             $(BUILD_DIR)/splitparstr.o \
             $(BUILD_DIR)/ec.o \
             $(BUILD_DIR)/bn254.o \
             $(BUILD_DIR)/bls12_381.o \
             $(BUILD_DIR)/fcall.o \
             $(BUILD_DIR)/arith256.o \
             $(BUILD_DIR)/arith384.o \
             $(BUILD_DIR)/add256.o \
             $(BUILD_DIR)/globals.o \
             $(BUILD_DIR)/goldilocks_base_field.o \
             $(BUILD_DIR)/poseidon2_goldilocks.o \
             $(BUILD_DIR)/blake2.o \
             $(BUILD_DIR)/keccak.o \
             $(BUILD_DIR)/bcon_sha256.o

ALL_OBJS = $(FIELD_ASM_OBJS) $(FIELD_CPP_OBJS) $(OTHER_OBJS)

# Header directories for include path
INCLUDES = -I$(SRC_DIR)

# Default target
all: $(LIB_FILE) $(BUILD_DIR)/clib

# Create directories
$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(LIB_DIR):
	mkdir -p $(LIB_DIR)

# Library target
$(LIB_FILE): $(ALL_OBJS) | $(LIB_DIR)
	ar rcs $@ $(ALL_OBJS)

# Test binary
$(BUILD_DIR)/clib: $(SRC_DIR)/main.cpp $(LIB_FILE)
	gcc $(CFLAGS) $(INCLUDES) $< -lc $(LIB_FILE) -o $@ -lgmp -lstdc++ -lgmpxx

# x86_64 NASM rules (used only when FIELD_BACKEND=asm_x86_64)
$(BUILD_DIR)/%_asm.o: $(SRC_DIR)/ffiasm/%.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

# ARM64 GAS rules (used only when FIELD_BACKEND=asm_arm64)
$(BUILD_DIR)/%_raw_arm64.o: $(SRC_DIR)/ffiasm/%_raw_arm64.s | $(BUILD_DIR)
	gcc $(CFLAGS) -c $< -o $@

# Field wrapper (.cpp) — always built
$(BUILD_DIR)/%_c.o: $(SRC_DIR)/ffiasm/%.cpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

# Field generic high-level (used by asm_arm64 and generic)
$(BUILD_DIR)/%_generic.o: $(SRC_DIR)/ffiasm/%_generic.cpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

# Field generic raw (used by generic backend)
$(BUILD_DIR)/%_raw_generic.o: $(SRC_DIR)/ffiasm/%_raw_generic.cpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

# Non-field C++ rules
$(BUILD_DIR)/alt_bn128.o: $(SRC_DIR)/ffiasm/alt_bn128.cpp $(SRC_DIR)/ffiasm/alt_bn128.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/secp256r1.o: $(SRC_DIR)/secp256r1/secp256r1.cpp $(SRC_DIR)/secp256r1/secp256r1.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/misc.o: $(SRC_DIR)/ffiasm/misc.cpp $(SRC_DIR)/ffiasm/misc.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/naf.o: $(SRC_DIR)/ffiasm/naf.cpp $(SRC_DIR)/ffiasm/naf.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/splitparstr.o: $(SRC_DIR)/ffiasm/splitparstr.cpp $(SRC_DIR)/ffiasm/splitparstr.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/ec.o: $(SRC_DIR)/ec/ec.cpp $(SRC_DIR)/ec/ec.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/bn254.o: $(SRC_DIR)/bn254/bn254.cpp $(SRC_DIR)/bn254/bn254.hpp $(SRC_DIR)/bn254/bn254_fe.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/bls12_381.o: $(SRC_DIR)/bls12_381/bls12_381.cpp $(SRC_DIR)/bls12_381/bls12_381.hpp $(SRC_DIR)/bls12_381/bls12_381_fe.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/fcall.o: $(SRC_DIR)/fcall/fcall.cpp $(SRC_DIR)/fcall/fcall.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/arith256.o: $(SRC_DIR)/arith256/arith256.cpp $(SRC_DIR)/arith256/arith256.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/arith384.o: $(SRC_DIR)/arith384/arith384.cpp $(SRC_DIR)/arith384/arith384.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/add256.o: $(SRC_DIR)/bigint/add256.cpp $(SRC_DIR)/bigint/add256.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/globals.o: $(SRC_DIR)/common/globals.cpp $(SRC_DIR)/common/globals.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/goldilocks_base_field.o: $(SRC_DIR)/poseidon2/goldilocks_base_field.cpp $(SRC_DIR)/poseidon2/goldilocks_base_field.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/poseidon2_goldilocks.o: $(SRC_DIR)/poseidon2/poseidon2_goldilocks.cpp $(SRC_DIR)/poseidon2/poseidon2_goldilocks.hpp $(SRC_DIR)/poseidon2/poseidon2_goldilocks_constants.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/blake2.o: $(SRC_DIR)/blake2/blake2.cpp $(SRC_DIR)/blake2/blake2.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/keccak.o: $(SRC_DIR)/chfast/keccak.c $(SRC_DIR)/chfast/keccak.h $(SRC_DIR)/chfast/hash_types.h $(SRC_DIR)/chfast/attributes.h | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/bcon_sha256.o: $(SRC_DIR)/bcon/bcon_sha256.c $(SRC_DIR)/bcon/bcon_sha256.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

clean:
	rm -rf $(BUILD_DIR)
	rm -rf $(LIB_DIR)

.PHONY: all clean
