# zisklib C bindings for the ZisK assembly library (ziskasm/zisklib/).
#
# Produces a small static library `zisklib_c` of redirectable stubs plus its
# public include dir: the `ziskos_*` flat-ABI stubs (`zisklib_stubs.c`, from
# <zisklib.h>) AND the Ethereum-Foundation `zkvm_*` accelerator stubs
# (`zkvm_stubs.c`, from <zkvm_accelerators.h> / <zkvm_u256.h>). A guest (e.g.
# ziskethone's cpp-guest) links it and calls either set; elf2rom redirects those
# symbols to the hand-written .zisk routines at transpile time.
#
# Two ways to consume it.
#
# 1. From a parent CMake project (in-tree):
#      add_subdirectory(/path/to/zisk/ziskasm/lang/c zisklib_c)
#      target_link_libraries(my_guest PRIVATE zisklib_c)
#
# 2. As a distributable archive (EF §9), built standalone and installed:
#      ./package.sh                       # -> dist/  (or see the install rules below)
#    which stages libzisklib_c.a, the public headers and the guest linker script
#    together. See README.md, "Packaging the static library".
#
# Notes:
#   - Build for the guest target (rv64ima) with the same toolchain as the guest.
#   - Do NOT strip the final ELF: elf2rom resolves the stubs by symbol name.
#   - Provides `_start` (src/_start.s), the EF I/O functions and every accelerator
#     stub, i.e. the whole guest-side surface of EF §9 in one archive, plus the
#     DMA-accelerated memcpy/memmove/memcmp/memset (EF §2), which _start.s defines
#     so they are always linked. Pair it with
#     a linker script that exports `_heap_start`/`_heap_end` (see
#     ziskbuild/zisk_linker_script.ld -- the single guest linker script).

cmake_minimum_required(VERSION 3.16)
project(zisklib_c C ASM)

# _start.s is part of the shipped library, not sample code: the EF standard (§9)
# requires the archive to provide the entry point alongside I/O and accelerators.
add_library(zisklib_c STATIC src/_start.s src/zisklib_stubs.c src/zkvm_stubs.c)
target_include_directories(zisklib_c PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
set_target_properties(zisklib_c PROPERTIES C_STANDARD 11 C_STANDARD_REQUIRED ON)
# Keep every stub as its own section so unused ones can be GC'd, while the
# referenced ones (the only ones that need redirecting) are retained with symbols.
target_compile_options(zisklib_c PRIVATE -ffunction-sections -O2)

# ---------------------------------------------------------------------------
# Install / packaging (EF §9: ship a .a + a linker script)
#
# Only when configured standalone -- a parent project that add_subdirectory()s us
# should not have our artifacts injected into its own `make install`.
# ---------------------------------------------------------------------------
if(CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR)
  include(GNUInstallDirs)

  install(TARGETS zisklib_c ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
  install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})

  # The linker script is not optional: the archive's _start expects the symbols it
  # defines (_global_pointer, _init_stack_top, __init_array_*, _heap_start/_end),
  # so the two have to travel together.
  set(ZISK_LINKER_SCRIPT "${CMAKE_CURRENT_SOURCE_DIR}/../../../ziskbuild/zisk_linker_script.ld")
  if(EXISTS "${ZISK_LINKER_SCRIPT}")
    install(FILES "${ZISK_LINKER_SCRIPT}" DESTINATION ${CMAKE_INSTALL_DATADIR}/zisk)
  else()
    message(WARNING
      "zisk_linker_script.ld not found at ${ZISK_LINKER_SCRIPT}; the installed "
      "archive will be unusable on its own. Copy the script in manually.")
  endif()
endif()
