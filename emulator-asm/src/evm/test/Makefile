# Standalone tests for the jump_dest assembly routines, in the shape of
# the DMA test harness next door.

CXX = g++
CXXFLAGS = -std=c++17 -O2 -Wall -Wextra -g -no-pie
LDFLAGS = -no-pie
ASMFLAGS = -g --noexecstack -I$(CURDIR) -I.. -I../../dma
BDIR = build

OBJS_ASM = $(BDIR)/jump_dest_fast.o \
           $(BDIR)/direct_jump_dest_mtrace.o \
           $(BDIR)/direct_jump_dest_mops.o \
           $(BDIR)/check_dynamic_mtrace.o \
           $(BDIR)/test_evm_api.o

OBJS = $(OBJS_ASM) test_jump_dest.o test_evm_mock.o

TARGET = test_jump_dest

all: $(TARGET)

$(BDIR)/%.o: ../%.asm ../jump_dest_macro.inc ../../dma/dma_constants.inc
	mkdir -p $(BDIR)
	as $(ASMFLAGS) -o $@ $<

$(BDIR)/check_dynamic_mtrace.o: ../../dma/check_dynamic_mtrace.asm ../../dma/dma_constants.inc
	mkdir -p $(BDIR)
	as $(ASMFLAGS) -o $@ $<

$(BDIR)/test_evm_api.o: test_evm_api.asm
	mkdir -p $(BDIR)
	as $(ASMFLAGS) -o $@ $<

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

$(TARGET): $(OBJS)
	$(CXX) $(LDFLAGS) -o $@ $^

test: $(TARGET)
	./$(TARGET)

clean:
	rm -rf $(BDIR) *.o $(TARGET)

.PHONY: all test clean
