# ZisK Sail model.
#
# MODEL is order-sensitive: Sail has no module system, definitions must
# precede use. Keep the prelude first and the step last.

SAIL ?= sail

# Sail memoizes z3 results in the working directory by default; keep it in build/.
SAILFLAGS = --memo-z3-path build/z3_problems

MODEL = model/zisk_prelude.sail \
        model/zisk_types.sail \
        model/zisk_ops.sail \
        model/zisk_inst.sail \
        model/zisk_step.sail

.PHONY: all check lean ops clean

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

clean:
	rm -rf build
