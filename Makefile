.PHONY: all clean

all:
	@echo "Use 'make clean' to remove build artifacts."

clean:
	cargo clean
	$(MAKE) -C lib-c/c clean