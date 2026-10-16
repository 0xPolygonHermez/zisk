# ZisK Sail model.
#
# MODEL is order-sensitive: Sail has no module system, definitions must
# precede use. Keep the prelude first and the step last.

SAIL ?= sail

# The AIR constraints come from the compiled pilout, not from the .pil sources.
# Build it with `tools/test-env/setup_build.sh --compile-pil` from the repo root.
PILOUT ?= ../pil/zisk.pilout
# Which AIRs to extract: one by name, or `--all` for every AIR in the pilout.
PIL_AIR ?= --air Main

# Both generated Lean trees are typechecked by the lake package in lean/, which
# points at them rather than holding sources of its own. Needs a Lean toolchain;
# install elan from https://lean-lang.org/install/.
LAKE ?= lake

# Sail memoizes z3 results in the working directory by default; keep it in build/.
SAILFLAGS = --memo-z3-path build/z3_problems

# The program: a file defining rom_fetch. The default has no instructions.
ROM ?= model/zisk_rom.sail

MODEL = model/zisk_prelude.sail \
        model/zisk_types.sail \
        model/zisk_mem.sail \
        model/zisk_ops.sail \
        model/zisk_inst.sail \
        $(ROM) \
        model/zisk_step.sail

# The C emulator, and the differential test that runs it against ziskemu.
# EMU_DIR holds one emulator per ROM, since the ROM is compiled in.
CC ?= cc
SAIL_LIB = $(shell $(SAIL) --dir)/lib
EMU_DIR ?= build/c

# The program to difftest, as `ziskemu -z` and `-i` take them.
PROG  ?= ../ziskasm/examples/doubler-min
INPUT ?= ../ziskasm/examples/doubler/input.bin
DIFF_DIR = build/difftest/$(notdir $(PROG))

.PHONY: all check lean ops pil lean-env lean-build pil-build proofs emu difftest clean

all: check ops

## Typecheck the model.
check:
	@mkdir -p build
	$(SAIL) $(SAILFLAGS) $(MODEL)

## Generate Lean definitions into build/lean/out.
lean:
	@mkdir -p build/lean
	$(SAIL) $(SAILFLAGS) --lean --lean-output-dir build/lean --lean-force-output $(MODEL)

## Fail if the model names an opcode that no longer exists in Rust.
ops:
	@python3 check_ops.py

## Generate Lean definitions of the AIR constraints into build/pil.
## PIL_AIR="--air Mem" picks another AIR, PIL_AIR=--all does every one.
pil:
	@test -f $(PILOUT) || { \
	  echo "$(PILOUT) not found — run tools/test-env/setup_build.sh --compile-pil" >&2; \
	  exit 1; }
	cargo run --release -p zisk-pilout-constraints -- \
	  lean --pilout $(PILOUT) $(PIL_AIR) --out build/pil

## Fail early with something actionable when the Lean setup is not usable.
lean-env:
	@command -v $(LAKE) >/dev/null 2>&1 || { \
	  echo "$(LAKE) not found — install elan: https://lean-lang.org/install/" >&2; \
	  exit 1; }
	@if [ -f build/lean/out/lean-toolchain ] && \
	   ! cmp -s build/lean/out/lean-toolchain lean/lean-toolchain; then \
	  echo "warning: lean/lean-toolchain is $$(cat lean/lean-toolchain)," >&2; \
	  echo "         but sail generated $$(cat build/lean/out/lean-toolchain)." >&2; \
	  echo "         Both libraries share one toolchain; copy sail's over it." >&2; \
	fi

## Typecheck the generated AIR constraints. Run `make pil` first.
pil-build: lean-env
	@test -d build/pil || { echo "build/pil is missing — run make pil" >&2; exit 1; }
	cd lean && $(LAKE) build Pil

## Check the proofs in lean/Proofs against the generated constraints.
proofs: lean-env
	@test -d build/pil || { echo "build/pil is missing — run make pil" >&2; exit 1; }
	cd lean && $(LAKE) build Proofs

## Typecheck the generated model. Run `make lean` first.
lean-build: lean-env
	@test -d build/lean/out || { echo "build/lean/out is missing — run make lean" >&2; exit 1; }
	cd lean && $(LAKE) build Out

## Build the C emulator for ROM into EMU_DIR/zisk_emu.
emu:
	@mkdir -p $(EMU_DIR)
	$(SAIL) $(SAILFLAGS) -c $(MODEL) model/zisk_main.sail -o $(EMU_DIR)/zisk_emu
	$(CC) -O2 -w -I $(SAIL_LIB) $(EMU_DIR)/zisk_emu.c $(SAIL_LIB)/*.c -lgmp -lz -o $(EMU_DIR)/zisk_emu

## Run PROG on ziskemu and on the model's C emulator, and diff the two traces.
## Silent beyond a summary when they agree; the first difference otherwise.
difftest:
	cargo run --release -q -p zisk-sail-difftest -- prepare -z $(PROG) \
	  $(if $(INPUT),-i $(INPUT)) -o $(DIFF_DIR)
	$(MAKE) --no-print-directory emu ROM=$(DIFF_DIR)/rom.sail EMU_DIR=$(DIFF_DIR)
	$(DIFF_DIR)/zisk_emu $$(cat $(DIFF_DIR)/mem.args) > $(DIFF_DIR)/sail.trace
	@diff -u $(DIFF_DIR)/ziskemu.trace $(DIFF_DIR)/sail.trace | head -40; \
	  if cmp -s $(DIFF_DIR)/ziskemu.trace $(DIFF_DIR)/sail.trace; then \
	    echo "OK: $$(grep -vc '^out' $(DIFF_DIR)/sail.trace) steps and the output agree with ziskemu"; \
	  else echo "FAIL: the model diverges from ziskemu (traces in $(DIFF_DIR))" >&2; exit 1; fi

clean:
	rm -rf build
	rm -rf lean/.lake
