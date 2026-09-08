# zisklib C bindings for the ZisK assembly library (ziskasm/zisklib/).
#
# Produces a small static library `zisklib_c` of redirectable `ziskos_*` stubs
# plus its public include dir. A guest (e.g. ziskethone's cpp-guest) links it and
# calls the `ziskos_*` functions from <zisklib.h>; elf2rom redirects those symbols
# to the hand-written .zisk routines at transpile time.
#
# Usage from a parent CMake project:
#   add_subdirectory(/path/to/zisk/ziskasm/lang/c zisklib_c)
#   target_link_libraries(my_guest PRIVATE zisklib_c)
#
# Notes:
#   - Build for the guest target (rv64ima) with the same toolchain as the guest.
#   - Do NOT strip the final ELF: elf2rom resolves the stubs by symbol name.

cmake_minimum_required(VERSION 3.16)
project(zisklib_c C)

add_library(zisklib_c STATIC src/zisklib_stubs.c)
target_include_directories(zisklib_c PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
set_target_properties(zisklib_c PROPERTIES C_STANDARD 11 C_STANDARD_REQUIRED ON)
# Keep every stub as its own section so unused ones can be GC'd, while the
# referenced ones (the only ones that need redirecting) are retained with symbols.
target_compile_options(zisklib_c PRIVATE -ffunction-sections -O2)
