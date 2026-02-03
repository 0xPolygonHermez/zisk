# Debug build flags
ifeq ($(dbg),1)
      CFLAGS = -g -D DEBUG -fPIC
      ASMFLAGS = -g
else
      CFLAGS = -O3 -fPIC
endif

# Directories
BUILD_DIR = build
LIB_DIR = lib
SRC_DIR = src

# Output library
LIB_NAME = libziskc.a
LIB_FILE = $(LIB_DIR)/$(LIB_NAME)

# Object files
ASM_OBJS = $(BUILD_DIR)/fec.o \
           $(BUILD_DIR)/fnec.o \
           $(BUILD_DIR)/fq.o \
           $(BUILD_DIR)/fr.o \
           $(BUILD_DIR)/bls12_381_384.o \
           $(BUILD_DIR)/bls12_381_asm.o \
           $(BUILD_DIR)/nsecp256r1.o \
           $(BUILD_DIR)/psecp256r1.o

CPP_OBJS = $(BUILD_DIR)/fecc.o \
           $(BUILD_DIR)/fnecc.o \
           $(BUILD_DIR)/fqc.o \
           $(BUILD_DIR)/frc.o \
           $(BUILD_DIR)/bls12_381_384c.o \
           $(BUILD_DIR)/bls12_381c.o \
           $(BUILD_DIR)/alt_bn128.o \
           $(BUILD_DIR)/nsecp256r1c.o \
           $(BUILD_DIR)/psecp256r1c.o \
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
           $(BUILD_DIR)/poseidon2_goldilocks.o

ALL_OBJS = $(ASM_OBJS) $(CPP_OBJS)

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

# Assembly rules
$(BUILD_DIR)/fec.o: $(SRC_DIR)/ffiasm/fec.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

$(BUILD_DIR)/fnec.o: $(SRC_DIR)/ffiasm/fnec.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

$(BUILD_DIR)/fq.o: $(SRC_DIR)/ffiasm/fq.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

$(BUILD_DIR)/fr.o: $(SRC_DIR)/ffiasm/fr.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

$(BUILD_DIR)/bls12_381_384.o: $(SRC_DIR)/ffiasm/bls12_381_384.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

$(BUILD_DIR)/bls12_381_asm.o: $(SRC_DIR)/ffiasm/bls12_381.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

$(BUILD_DIR)/nsecp256r1.o: $(SRC_DIR)/ffiasm/nsecp256r1.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

$(BUILD_DIR)/psecp256r1.o: $(SRC_DIR)/ffiasm/psecp256r1.asm | $(BUILD_DIR)
	nasm -felf64 $< -o $@

# C++ compilation rules
$(BUILD_DIR)/fecc.o: $(SRC_DIR)/ffiasm/fec.cpp $(SRC_DIR)/ffiasm/fec.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/fnecc.o: $(SRC_DIR)/ffiasm/fnec.cpp $(SRC_DIR)/ffiasm/fnec.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/fqc.o: $(SRC_DIR)/ffiasm/fq.cpp $(SRC_DIR)/ffiasm/fq.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/frc.o: $(SRC_DIR)/ffiasm/fr.cpp $(SRC_DIR)/ffiasm/fr.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/bls12_381_384c.o: $(SRC_DIR)/ffiasm/bls12_381_384.cpp $(SRC_DIR)/ffiasm/bls12_381_384.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/bls12_381c.o: $(SRC_DIR)/ffiasm/bls12_381.cpp $(SRC_DIR)/ffiasm/bls12_381.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/alt_bn128.o: $(SRC_DIR)/ffiasm/alt_bn128.cpp $(SRC_DIR)/ffiasm/alt_bn128.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/nsecp256r1c.o: $(SRC_DIR)/ffiasm/nsecp256r1.cpp $(SRC_DIR)/ffiasm/nsecp256r1.hpp | $(BUILD_DIR)
	gcc $(CFLAGS) $(INCLUDES) -c $< -o $@

$(BUILD_DIR)/psecp256r1c.o: $(SRC_DIR)/ffiasm/psecp256r1.cpp $(SRC_DIR)/ffiasm/psecp256r1.hpp | $(BUILD_DIR)
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

clean:
	rm -rf $(BUILD_DIR)
	rm -rf $(LIB_DIR)

.PHONY: all clean