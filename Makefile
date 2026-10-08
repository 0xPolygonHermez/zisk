.DEFAULT_GOAL := all

.PHONY: all clean libziskos

all:
	@echo "Use 'make clean' to remove build artifacts."
	@echo "Use 'make libziskos' to build the isolated ziskos staticlib."

# Build the isolated ziskos staticlib (libziskos.a). Discoverable, dev-facing
# wrapper around the CI script in .github/scripts. Enable cargo features with
# FEATURES, e.g. `make libziskos FEATURES=alloc-stats`.
libziskos:
	ZISK_FEATURES="$(FEATURES)" .github/scripts/build-libziskos-isolated.sh

# `cargo clean` removes all real artifacts (lib-c builds under target/ now).
# The lib-c/c clean only scrubs legacy in-source build/lib dirs from old trees.
clean:
	cargo clean
	$(MAKE) -C lib-c/c clean
	rm -rf test-artifacts/programs/diagnostic/target
	rm -rf test-artifacts/programs/target
	rm -rf test-artifacts/target