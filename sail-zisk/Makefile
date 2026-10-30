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

# The ROM module, a file defining rom_fetch, relative to model/. The default
# has no instructions, for the typecheck and the Lean build.
ROM ?= zisk_rom.sail
ROMFLAG = --variable 'ROM=$(ROM)'

# The C emulator: one for every program, reading the program's ROM from the
# table tools/sail-difftest writes (model/zisk_rom_table.sail). Rebuilt when
# the model changes.
EMU = build/c/zisk_emu
MODEL_SRCS = $(PROJECT) $(wildcard model/*.sail)

# The program to difftest: ZisK assembly (PROG, as `ziskemu -z` takes it, or
# several of them, assembled together) or
# a RISC-V ELF (ELF, as `ziskemu -e` takes it, with NAME for its directory),
# and its input, as `ziskemu -i` takes it.
PROG  ?= ../ziskasm/examples/doubler-min
INPUT ?= ../ziskasm/examples/doubler/input.bin
ELF   ?=
# A memory range to trace after the run, `<begin>,<end>` in hex: a test's
# signature, say.
DUMP  ?=
# Set for a program ZisK must refuse: the runs must both stop on a fault.
EXPECT_FAULT ?=
NAME  ?= $(if $(ELF),$(basename $(notdir $(ELF))),$(notdir $(firstword $(PROG))))
DIFF_DIR = build/difftest/$(NAME)

.PHONY: all check ops compliance transpiler-check decoder-diff lean lean-build riscv riscv-sim riscv-lean riscv-run zisk-riscv equiv proofs emu difftest difftest-all constants optest opsdoc sail-version clean

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

## Fail if the model names an opcode that no longer exists in Rust, numbers
## one differently, or lacks one: every operation is specified; or if the
## operations' attributes are stale.
ops:
	@python3 check_ops.py --strict
	@python3 gen_op_attrs.py --check

## Generate Lean definitions of the model (without the C main) into build/lean.
lean: sail-version
	@mkdir -p build/lean
	$(SAIL) $(SAILFLAGS) $(ROMFLAG) --project $(PROJECT) zisk_step \
	  --lean --lean-output-dir build/lean --lean-force-output -o Zisk

## Elaborate the model's generated Lean (build/lean/Zisk). Run `make lean` first.
lean-build:
	@test -d build/lean/Zisk || { echo "build/lean/Zisk is missing — run make lean" >&2; exit 1; }
	cd build/lean/Zisk && $(LAKE) update && $(LAKE) build

## sail-riscv's configuration for the RISC-V ZisK runs (riscv/zisk_profile.py),
## from the default its CMake configure writes.
ZISK_RV_CONFIG = build/riscv/zisk_rv64.json
$(ZISK_RV_CONFIG): riscv/zisk_profile.py $(SAIL_RISCV_DIR)/build/config/rv64d_v256_e64.json
	@mkdir -p build/riscv
	python3 riscv/zisk_profile.py $(SAIL_RISCV_DIR)/build/config/rv64d_v256_e64.json $@

## Fetch sail-riscv at the pinned commit, configure it, and generate its Lean
## for ZisK's profile (build/riscv-lean/Lean_ZiskRV): sail-riscv's own Lean
## command (its generated_lean_rv64d target), with ZisK's configuration, which
## the Lean backend fixes at generation time, and unchanged sources. ~5 min.
RISCV_LEAN = build/riscv-lean/Lean_ZiskRV
riscv: sail-version
	@if [ ! -d $(SAIL_RISCV_DIR)/.git ]; then git clone -q $(SAIL_RISCV_URL) $(SAIL_RISCV_DIR); fi
	cd $(SAIL_RISCV_DIR) && git fetch -q origin && git checkout -q $(SAIL_RISCV_REV)
	$(CMAKE) -S $(SAIL_RISCV_DIR) -B $(SAIL_RISCV_DIR)/build -DCMAKE_BUILD_TYPE=RelWithDebInfo > /dev/null
	$(MAKE) --no-print-directory $(ZISK_RV_CONFIG)
	@mkdir -p build/riscv-lean
	cd $(SAIL_RISCV_DIR)/model && $(SAIL) --strict-var --strict-bitvector --strict-exponentials \
	  --require-version $(SAIL_VERSION) --memo-z3-path $(abspath $(SAIL_RISCV_DIR))/build/model/sail_smt_cache \
	  --config $(abspath $(ZISK_RV_CONFIG)) --lean --memo-z3 --lean-output-dir $(abspath build/riscv-lean) \
	  --lean-force-output --lean-non-beq-type instruction --lean-non-beq-type instr_info \
	  --lean-non-beq-type HartState --lean-non-beq-type ExecutionResult --lean-non-beq-type Step \
	  --lean-noncomputable $(foreach f,encdec_forwards encdec_backwards encdec_forwards_matches \
	    encdec_backwards_matches encdec_compressed_forwards encdec_compressed_backwards \
	    encdec_compressed_forwards_matches encdec_compressed_backwards_matches,--lean-noncomputable-function $(f)) \
	  --lean-import-file ../handwritten_support/RiscvExtras.lean -o Lean_ZiskRV --all-modules riscv.sail_project
	python3 riscv/gen_lean_regions.py build/riscv-lean/Lean_ZiskRV/LeanZiskRV.lean lean/Equiv/Regions.lean

## Build sail-riscv's C emulator, sail_riscv_sim, for `make compliance` to
## compare signatures with, in ZisK's profile. Run `make riscv` first; long
## (~15 min).
RISCV_SIM ?= $(SAIL_RISCV_DIR)/build/c_emulator/sail_riscv_sim
riscv-sim: $(ZISK_RV_CONFIG)
	$(CMAKE) --build $(SAIL_RISCV_DIR)/build --target sail_riscv_sim
	$(RISCV_SIM) --config $(ZISK_RV_CONFIG) --validate-config

## Elaborate sail-riscv's generated Lean. Run `make riscv` first; long (~12 min).
riscv-lean:
	cd $(RISCV_LEAN) && $(LAKE) update && $(LAKE) build

## ZisK's RISC-V machine, runnable (lean/ZiskRiscvRun): sail-riscv's executable
## Lean in ZisK's profile, and zisk_riscv_run on it. Lean's compiled runtime
## takes an inductive of at most ~256 constructors, which sail-riscv's
## instruction type exceeds with every module, so the executable model has the
## modules its Lean emulator's README lists, and ZisK's profile's — the
## leaves: selecting a module does not select the modules nested in it — the
## same semantics for every instruction the profile has. Run `make riscv` first;
## long (~15 min).
RISCV_RUN_MODULES = H Zca Zicbom_insts Zicboz Zicsr_insts Zicfiss_regs Zimop Zaamo Zicntr Zkr \
  Sscofpmf Ssqosid postlude I_insts M_insts Zalrsc Zba Zbb Zbc Zbs Zbkb Zifencei
ZISK_RISCV_RUN = lean/.lake/build/bin/zisk_riscv_run
riscv-run: $(ZISK_RV_CONFIG)
	cd $(SAIL_RISCV_DIR)/model && $(SAIL) --strict-var --strict-bitvector --strict-exponentials \
	  --require-version $(SAIL_VERSION) --memo-z3-path $(abspath $(SAIL_RISCV_DIR))/build/model/sail_smt_cache \
	  --config $(abspath $(ZISK_RV_CONFIG)) --lean --memo-z3 --lean-output-dir $(abspath build/riscv-lean) \
	  --lean-force-output --lean-non-beq-type instruction --lean-non-beq-type instr_info \
	  --lean-non-beq-type HartState --lean-non-beq-type ExecutionResult --lean-non-beq-type Step \
	  --lean-import-file ../handwritten_support/RiscvExtrasExecutable.lean -o Lean_ZiskRV_executable \
	  $(RISCV_RUN_MODULES) riscv.sail_project
	python3 riscv/gen_lean_registers.py $(SAIL_RISCV_DIR) \
	  build/riscv-lean/Lean_ZiskRV_executable/LeanZiskRVExecutable/Defs.lean lean/ZiskRiscvRun/Registers.lean
	cd lean && $(LAKE) update && $(LAKE) build zisk_riscv_run

## Run ELF, after `make difftest ELF=...` has prepared it, on ZisK's RISC-V
## machine too, from the same memory but the ROM table (0x1_xxxx_xxxx_xxxx,
## which only sail-zisk reads), and diff how it ends — the exit, the output, the dump range —
## with sail-zisk's run of its transpilation.
zisk-riscv:
	@test -x $(ZISK_RISCV_RUN) || { echo "$(ZISK_RISCV_RUN) is missing — run make riscv-run" >&2; exit 1; }
	$(ZISK_RISCV_RUN) $(ELF) $$(grep -vE '^-b 0x1[0-9a-f]{12},' $(DIFF_DIR)/mem.args) \
	  $(if $(DUMP),--dump $(DUMP)) > $(DIFF_DIR)/riscv.end
	@grep -E '^(end|out|dump) ' $(DIFF_DIR)/sail.trace | diff -u - $(DIFF_DIR)/riscv.end | head -20; \
	  if grep -E '^(end|out|dump) ' $(DIFF_DIR)/sail.trace | cmp -s - $(DIFF_DIR)/riscv.end; then \
	    echo "OK: ZisK's RISC-V machine ends as sail-zisk does: exit, output$(if $(DUMP), and dump)"; \
	  else echo "FAIL: ZisK's RISC-V machine ends otherwise than sail-zisk (in $(DIFF_DIR))" >&2; exit 1; fi

## Build the equivalence package in lean/, which imports both models. Run
## `make lean` and `make riscv` first.
equiv:
	cd lean && $(LAKE) update && $(LAKE) build

## The proofs: build lean/ (make equiv), then check the axioms the theorems,
## zisk_total and zisk_totalX, rest on: Lean's, bv_decide's, and those sail-riscv's Lean
## leaves undefined (lean/axioms.txt); no `sorry`. Run `make lean`, `make
## riscv` and `make riscv-run` first. ~15 min on 128 cores.
proofs: equiv
	@mkdir -p build
	for t in zisk_total zisk_totalX; do printf 'import Equiv\n#print axioms Equiv.Final.%s\n' $$t | (cd lean && $(LAKE) env lean --stdin) > build/axioms-$$t.out && python3 lean/check_axioms.py lean/axioms.txt build/axioms-$$t.out || exit 1; done

## Build the C emulator into build/c/zisk_emu.
emu: $(EMU)

$(EMU): $(MODEL_SRCS) | sail-version
	@mkdir -p build/c
	$(SAIL) $(SAILFLAGS) --variable 'ROM=zisk_rom_table.sail' --project $(PROJECT) main -c -o $@
	$(CC) -O2 -w -I $(SAIL_LIB) $@.c $(SAIL_LIB)/*.c -lgmp -lz -o $@

## Run PROG on ziskemu and on the model's C emulator, and diff the two traces.
difftest: $(EMU)
	cargo run --release -q -p zisk-sail-difftest -- prepare $(if $(ELF),-e $(ELF),$(foreach p,$(PROG),-z $(p))) \
	  $(if $(INPUT),-i $(INPUT)) $(if $(DUMP),--dump $(DUMP)) -o $(DIFF_DIR)
	$(EMU) $$(cat $(DIFF_DIR)/mem.args) > $(DIFF_DIR)/sail.trace 2> $(DIFF_DIR)/sail.err; \
	  echo $$? > $(DIFF_DIR)/sail.status
	@python3 test/compare_traces.py $(DIFF_DIR) $(if $(EXPECT_FAULT),--expect-fault)

## The RISC-V architectural tests, transpiled: every test of SUITES compiled
## for ZisK's memory map and difftested, and, once `make riscv-sim` has built
## sail-riscv's emulator (RISCV_SIM), its signature compared with sail-riscv's.
## Needs a riscv-arch-test checkout (ARCH_TEST) and a RISC-V GCC (RISCV_GCC).
## See test/compliance/run.py.
ARCH_TEST ?=
RISCV_GCC ?= riscv-none-elf-gcc
SUITES    ?= I M A B K
compliance:
	python3 test/compliance/run.py --arch-test '$(ARCH_TEST)' --gcc '$(RISCV_GCC)' \
	  $(if $(wildcard $(RISCV_SIM)),--riscv-sim '$(RISCV_SIM)') \
	  $(if $(and $(wildcard $(RISCV_SIM)),$(wildcard $(ZISK_RISCV_RUN))),--zisk-riscv) $(SUITES)

## The transpiler's spec in Lean (lean/Transpiler/Spec.lean) against the
## transpiler: for each RISC-V ELF of TRANSPILER_ELFS, the ROM the Rust builds
## and what the spec gives, compared instruction by instruction, field by
## field (lean/TranspilerCheck). By default the ELFs `make compliance` built,
## for SUITES; an ELF the transpiler rejects (Zbc's) is skipped, and said so.
## With them, the hand-written programs of test/transpiler (the CSR forms of
## the contract's §§ 6–8, which the suites lack), when RISCV_GCC is there.
TRANSPILER_TESTS := $(if $(shell command -v $(RISCV_GCC) 2>/dev/null),$(patsubst test/transpiler/%.S,build/transpiler/%.elf,$(wildcard test/transpiler/*.S)))
TRANSPILER_ELFS ?= $(foreach s,$(SUITES),$(wildcard build/compliance/$(s)/*.elf)) $(TRANSPILER_TESTS)
build/transpiler/%.elf: test/transpiler/%.S test/compliance/env/link.ld
	@mkdir -p $(dir $@)
	$(RISCV_GCC) -march=rv64ima_zicsr -mabi=lp64 -nostdlib -T test/compliance/env/link.ld -o $@ $< 2>/dev/null \
	  || $(RISCV_GCC) -march=rv64ima -mabi=lp64 -nostdlib -T test/compliance/env/link.ld -o $@ $<
transpiler-check: $(TRANSPILER_TESTS)
	cargo build --release -q -p zisk-sail-difftest
	cd lean && $(LAKE) build transpiler_check
	@rm -rf build/transpiler-check && mkdir -p build/transpiler-check
	@for e in $(TRANSPILER_ELFS); do \
	  n=$$(basename $$(dirname $$e))-$$(basename $$e .elf); \
	  ../target/release/sail-difftest rom -e $$e -o build/transpiler-check/$$n.rom > /dev/null 2>&1 \
	    || echo "skip $$e: the transpiler rejects it"; \
	done
	lean/.lake/build/bin/transpiler_check build/transpiler-check/*.rom

## The transpiler's decoder against sail-riscv's, in ZisK's profile: every
## word the transpiler's decoder can tell apart (opcode, funct3, bits 31:20,
## rd and rs1 x0 or not), what the transpiler does with it against what
## sail-riscv decodes it to; fails where they differ outside what the
## contract allows. Run `make riscv-run` first: sail-riscv's side runs on its
## executable Lean (lean/DecodeDiff). ~11 min.
decoder-diff:
	cargo build --release -q -p zisk-sail-difftest
	cd lean && $(LAKE) build decode_diff
	@mkdir -p build/decoder-diff
	lean/.lake/build/bin/decode_diff build/decoder-diff/sail.bin build/decoder-diff/names.txt
	@../target/release/sail-difftest decode-diff --sail build/decoder-diff/sail.bin \
	  --names build/decoder-diff/names.txt > build/decoder-diff/report.txt; \
	  status=$$?; cat build/decoder-diff/report.txt; exit $$status

## Every program the model is difftested on.
# ziskasm's diagnostic program: every family of operations, each checked
# in-program; output[0] = 0 when all pass.
DIAG = ../ziskasm/programs/diagnostic
# Random operands for every precompile: how many per precompile, and the seed.
PRECOMP_VECTORS ?= 8
PRECOMP_SEED    ?= 1

difftest-all:
	$(MAKE) --no-print-directory difftest
	$(MAKE) --no-print-directory difftest PROG=test/programs/halt INPUT=
	@grep -q '^end 1 0x0000000000000003$$' build/difftest/halt/sail.trace || { \
	  echo "FAIL: test/programs/halt no longer ends in error with code 3 — has the BIOS's mtvec moved?" >&2; exit 1; }
	$(MAKE) --no-print-directory difftest PROG=test/programs/memops INPUT=
	$(MAKE) --no-print-directory difftest PROG=$(DIAG) INPUT=
	@grep -q '^out 0 0x0000000000000000$$' build/difftest/diagnostic/sail.trace || { \
	  echo "FAIL: ziskasm's diagnostic reports an error (output[0] in build/difftest/diagnostic)" >&2; exit 1; }
	$(MAKE) --no-print-directory difftest PROG=test/programs/memmap/edges.zisk NAME=memmap-edges INPUT=
	@for p in $(filter-out %/edges.zisk,$(wildcard test/programs/memmap/*.zisk)); do \
	  $(MAKE) --no-print-directory difftest PROG=$$p NAME=memmap-$$(basename $$p .zisk) INPUT= EXPECT_FAULT=1 || exit 1; \
	done
	@mkdir -p build/precompiles
	python3 test/gen_precompile_test.py build/precompiles/precompiles.zisk $(PRECOMP_VECTORS) $(PRECOMP_SEED)
	$(MAKE) --no-print-directory difftest PROG=build/precompiles INPUT=

## Regenerate what the model takes from the Rust sources: the operations'
## attributes and the Poseidon constants.
constants:
	python3 gen_op_attrs.py
	python3 gen_poseidon_constants.py

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
