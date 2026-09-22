# CMake toolchain file for building ZisK guest artifacts (bare-metal RV64).
#
#   cmake -S . -B build -DCMAKE_TOOLCHAIN_FILE=$PWD/zisk-guest-toolchain.cmake
#
# Override the tool prefix for the xPack toolchain:
#   -DZISK_TOOLCHAIN_PREFIX=riscv-none-elf-
#
# The arch string is deliberately rv64ima and NOT rv64imac. ZisK decodes the
# compressed (C) extension only when built with the `compressed` cargo feature,
# which is OFF by default, so a default ZisK is IALIGN = 32 and rejects a guest
# containing 16-bit instructions. Building the archive without C keeps it usable
# by the default toolchain; a consumer who enables `compressed` can override
# ZISK_GUEST_ARCH.

set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR riscv64)

if(NOT DEFINED ZISK_TOOLCHAIN_PREFIX)
  set(ZISK_TOOLCHAIN_PREFIX "riscv64-unknown-elf-")
endif()
if(NOT DEFINED ZISK_GUEST_ARCH)
  set(ZISK_GUEST_ARCH "-march=rv64ima -mabi=lp64 -mcmodel=medany")
endif()

set(CMAKE_C_COMPILER   ${ZISK_TOOLCHAIN_PREFIX}gcc)
set(CMAKE_CXX_COMPILER ${ZISK_TOOLCHAIN_PREFIX}g++)
set(CMAKE_ASM_COMPILER ${ZISK_TOOLCHAIN_PREFIX}gcc)
set(CMAKE_AR      ${ZISK_TOOLCHAIN_PREFIX}ar     CACHE FILEPATH "archiver")
set(CMAKE_RANLIB  ${ZISK_TOOLCHAIN_PREFIX}ranlib CACHE FILEPATH "ranlib")

set(CMAKE_C_FLAGS_INIT   "${ZISK_GUEST_ARCH} -ffreestanding")
set(CMAKE_CXX_FLAGS_INIT "${ZISK_GUEST_ARCH} -ffreestanding")
set(CMAKE_ASM_FLAGS_INIT "${ZISK_GUEST_ARCH}")

# There is no libc and no default linker script, so CMake's compiler check must
# not try to link an executable.
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM BEFORE)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
