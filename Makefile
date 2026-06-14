.DEFAULT_GOAL := all
 
.PHONY: all clean

all:
	@echo "Use 'make clean' to remove build artifacts."

# `cargo clean` removes all real artifacts (lib-c builds under target/ now).
# The lib-c/c clean only scrubs legacy in-source build/lib dirs from old trees.
clean:
	cargo clean
	$(MAKE) -C lib-c/c clean