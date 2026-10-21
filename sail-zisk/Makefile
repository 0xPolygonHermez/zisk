# The ZisK Sail model: its C emulator, its Lean definitions, and the
# differential test against ziskemu.
#
# The model's modules are in model/zisk.sail_project, as sail-riscv lays out
# RISC-V in riscv.sail_project.

SAIL ?= sail
CC   ?= cc
LAKE ?= lake
CMAKE ?= cmake

# sail-riscv, the RISC-V model the equivalence is proved against, pinned: the
# commit's Lean output is the one `make riscv` generates and builds. Move the
# pin only together with SAIL_VERSION, which that commit requires.
SAIL_VERSION    = 0.20.3
SAIL_RISCV_URL ?= https://github.com/riscv/sail-riscv
SAIL_RISCV_REV  = 92079109706b802ead111cbe0be173f7917d5faa
SAIL_RISCV_DIR ?= build/sail-riscv
SAIL_LIB = $(shell $(SAIL) --dir)/lib
PROJECT = model/zisk.sail_project

# Sail memoizes z3 results in the working directory by default; keep it in build/.
SAILFLAGS = --memo-z3-path $(CURDIR)/build/z3_problems

# The program: a file defining rom_fetch, relative to model/. The default has
# no instructions.
ROM ?= zisk_rom.sail
ROMFLAG = --variable 'ROM=$(ROM)'

# The C emulator; EMU_DIR holds one per ROM, since the ROM is compiled in.
EMU_DIR ?= build/c

# The program to difftest: ZisK assembly (PROG, as `ziskemu -z` takes it) or
# a RISC-V ELF (ELF, as `ziskemu -e` takes it, with NAME for its directory),
# and its input, as `ziskemu -i` takes it.
PROG  ?= ../ziskasm/examples/doubler-min
INPUT ?= ../ziskasm/examples/doubler/input.bin
ELF   ?=
NAME  ?= $(if $(ELF),$(basename $(notdir $(ELF))),$(notdir $(PROG)))
DIFF_DIR = build/difftest/$(NAME)

.PHONY: all check ops lean lean-build riscv riscv-lean equiv emu difftest difftest-all optest opsdoc sail-version clean

all: check ops

## Fail early, and say so, when the installed Sail is not the pinned one.
sail-version:
	@v=$$($(SAIL) --version | sed -n 's/^Sail \([0-9.]*\).*/\1/p'); \
	  if [ "$$v" != "$(SAIL_VERSION)" ]; then \
	    echo "Sail $(SAIL_VERSION) is required, found $$v — opam install sail.$(SAIL_VERSION)" >&2; exit 1; fi

## Typecheck the model, with the C emulator's main.
check: sail-version
	@mkdir -p build
	$(SAIL) $(SAILFLAGS) $(ROMFLAG) --project $(PROJECT) main

## Fail if the model names an opcode that no longer exists in Rust.
ops:
	@python3 check_ops.py

## Generate Lean definitions of the model (without the C main) into build/lean.
lean: sail-version
	@mkdir -p build/lean
	$(SAIL) $(SAILFLAGS) $(ROMFLAG) --project $(PROJECT) zisk_step \
	  --lean --lean-output-dir build/lean --lean-force-output -o Zisk

## Elaborate the model's generated Lean (build/lean/Zisk). Run `make lean` first.
lean-build:
	@test -d build/lean/Zisk || { echo "build/lean/Zisk is missing — run make lean" >&2; exit 1; }
	cd build/lean/Zisk && $(LAKE) update && $(LAKE) build

## Fetch sail-riscv at the pinned commit and generate its RV64 Lean
## (SAIL_RISCV_DIR/build/model/Lean_RV64D), as its own CMake target does.
riscv: sail-version
	@if [ ! -d $(SAIL_RISCV_DIR)/.git ]; then git clone -q $(SAIL_RISCV_URL) $(SAIL_RISCV_DIR); fi
	cd $(SAIL_RISCV_DIR) && git fetch -q origin && git checkout -q $(SAIL_RISCV_REV)
	$(CMAKE) -S $(SAIL_RISCV_DIR) -B $(SAIL_RISCV_DIR)/build -DCMAKE_BUILD_TYPE=RelWithDebInfo > /dev/null
	$(CMAKE) --build $(SAIL_RISCV_DIR)/build --target generated_lean_rv64d

## Elaborate sail-riscv's generated Lean. Run `make riscv` first; long (~12 min).
riscv-lean:
	cd $(SAIL_RISCV_DIR)/build/model/Lean_RV64D && $(LAKE) update && $(LAKE) build

## Build the equivalence package in lean/, which imports both models. Run
## `make lean` and `make riscv` first.
equiv:
	cd lean && $(LAKE) update && $(LAKE) build

## Build the C emulator for ROM into EMU_DIR/zisk_emu.
emu: sail-version
	@mkdir -p $(EMU_DIR)
	$(SAIL) $(SAILFLAGS) $(ROMFLAG) --project $(PROJECT) main -c -o $(EMU_DIR)/zisk_emu
	$(CC) -O2 -w -I $(SAIL_LIB) $(EMU_DIR)/zisk_emu.c $(SAIL_LIB)/*.c -lgmp -lz -o $(EMU_DIR)/zisk_emu

## Run PROG on ziskemu and on the model's C emulator, and diff the two traces.
difftest:
	cargo run --release -q -p zisk-sail-difftest -- prepare $(if $(ELF),-e $(ELF),-z $(PROG)) \
	  $(if $(INPUT),-i $(INPUT)) -o $(DIFF_DIR)
	$(MAKE) --no-print-directory emu ROM=../$(DIFF_DIR)/rom.sail EMU_DIR=$(DIFF_DIR)
	$(DIFF_DIR)/zisk_emu $$(cat $(DIFF_DIR)/mem.args) > $(DIFF_DIR)/sail.trace
	@diff -u $(DIFF_DIR)/ziskemu.trace $(DIFF_DIR)/sail.trace | head -40; \
	  if cmp -s $(DIFF_DIR)/ziskemu.trace $(DIFF_DIR)/sail.trace; then \
	    echo "OK: $$(grep -vEc '^(out|end) ' $(DIFF_DIR)/sail.trace) steps and the output agree with ziskemu"; \
	  else echo "FAIL: the model diverges from ziskemu (traces in $(DIFF_DIR))" >&2; exit 1; fi

## Every program the model is difftested on.
difftest-all:
	$(MAKE) --no-print-directory difftest
	$(MAKE) --no-print-directory difftest PROG=test/programs/halt INPUT=
	@grep -q '^end 1 0x0000000000000003$$' build/difftest/halt/sail.trace || { \
	  echo "FAIL: test/programs/halt no longer ends in error with code 3 — has the BIOS's mtvec moved?" >&2; exit 1; }

## Check every operation the model specifies against ZisK's own op functions
## (ZiskOp::call_ab) on edge-case and random vectors. The checks run on the
## model's operations alone: the files of its zisk_insts module, then the
## test's. See test/gen_op_checks.py.
OPS_FILES = $(shell $(SAIL) --project $(PROJECT) zisk_insts --list-files)
optest:
	@mkdir -p build/optest
	cargo run --release -q -p zisk-sail-difftest -- ops -o build/optest/vectors.txt
	python3 test/gen_op_checks.py build/optest/vectors.txt build/optest/op_checks.sail
	$(SAIL) $(SAILFLAGS) $(OPS_FILES) test/optest_check.sail build/optest/op_checks.sail \
	  test/optest_main.sail -c -o build/optest/optest
	$(CC) -O1 -w -I $(SAIL_LIB) build/optest/optest.c $(SAIL_LIB)/*.c -lgmp -lz -o build/optest/optest
	build/optest/optest | tee build/optest/result.txt
	@grep -q '^OK' build/optest/result.txt

## Regenerate doc/ops.md, the operations' coverage. See doc/gen_ops_doc.py.
opsdoc:
	python3 doc/gen_ops_doc.py

clean:
	rm -rf build
