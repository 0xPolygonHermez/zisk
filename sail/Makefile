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

# Sail memoizes z3 results in the working directory by default; keep it in build/.
SAILFLAGS = --memo-z3-path build/z3_problems

MODEL = model/zisk_prelude.sail \
        model/zisk_types.sail \
        model/zisk_ops.sail \
        model/zisk_inst.sail \
        model/zisk_step.sail

.PHONY: all check lean ops pil clean

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

clean:
	rm -rf build
