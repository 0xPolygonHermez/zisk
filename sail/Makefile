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

MODEL = model/zisk_prelude.sail \
        model/zisk_types.sail \
        model/zisk_ops.sail \
        model/zisk_inst.sail \
        model/zisk_step.sail

.PHONY: all check lean ops pil lean-env lean-build pil-build proofs clean

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
## Fails today: the model declares rX, wX, read_mem, write_mem and rom_fetch
## without definitions, so Out.ZiskStep has no Lean to elaborate.
lean-build: lean-env
	@test -d build/lean/out || { echo "build/lean/out is missing — run make lean" >&2; exit 1; }
	cd lean && $(LAKE) build Out

clean:
	rm -rf build
	rm -rf lean/.lake
