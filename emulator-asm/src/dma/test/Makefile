# Makefile for encode_memcpy_inline test

CXX = g++
CXXFLAGS = -std=c++17 -O0 -Wall -Wextra -g -no-pie -I../../../../state-machines/mem-cpp/cpp
LDFLAGS = -no-pie
ASMFLAGS = -g --noexecstack -I$(CURDIR) -I.. 
BDIR = build

# Output directory for object files (current directory)
OBJDIR = .

SRCS_ASM = ../direct_memcpy_mtrace.asm ../memcpy_fast.asm ../fast_memcpy.asm ../direct_memcpy_mops.asm \
	       ../direct_memset_mops.asm ../direct_memset_mtrace.asm ../fast_memset.asm  ../fast_memcpy64.asm \
 		   ../direct_inputcpy_mops.asm ../direct_inputcpy_mtrace.asm ../fast_dma_encode.asm \
		   ../check_dynamic_mtrace.asm ../direct_memcmp_mops.asm ../direct_memcmp_mtrace.asm \
		   ../fast_memcmp.asm ../fast_inputcpy.asm ../test_dma_api.asm --

SRCS_CPP = test_dma.cpp test_dma_encode.cpp test_dma_tools.cpp test_mock.cpp \
		   test_dma_memcmp_mtrace.cpp test_dma_memcmp_mops.cpp \
		   test_dma_memcpy_mtrace.cpp test_dma_memcpy_mops.cpp \
		   test_dma_memset_mtrace.cpp test_dma_memset_mops.cpp \
		   test_dma_inputcpy_mtrace.cpp test_dma_inputcpy_mops.cpp \
		   test_dma_mem.cpp test_dma_mem_mtrace.cpp test_dma_mem_mops.cpp



OBJS_ASM = $(BDIR)/direct_memcpy_mtrace.o $(BDIR)/memcpy_fast.o $(BDIR)/fast_memcpy.o \
		   $(BDIR)/direct_memcpy_mops.o $(BDIR)/direct_memset_mops.o $(BDIR)/direct_memset_mtrace.o \
		   $(BDIR)/fast_memset.o  $(BDIR)/fast_memcpy64.o $(BDIR)/direct_inputcpy_mops.o \
		   $(BDIR)/direct_inputcpy_mtrace.o $(BDIR)/fast_dma_encode.o $(BDIR)/check_dynamic_mtrace.o \
		   $(BDIR)/direct_memcmp_mops.o $(BDIR)/direct_memcmp_mtrace.o $(BDIR)/fast_memcmp.o \
		   $(BDIR)/fast_inputcpy.o $(BDIR)/test_dma_api.o

OBJS = $(OBJS_ASM) test_dma.o test_dma_encode.o test_dma_tools.o test_mock.o \
		test_dma_memcmp_mtrace.o test_dma_memcmp_mops.o \
		test_dma_memcpy_mtrace.o test_dma_memcpy_mops.o \
		test_dma_memset_mtrace.o test_dma_memset_mops.o \
		test_dma_inputcpy_mtrace.o test_dma_inputcpy_mops.o \
		test_dma_mem.o test_dma_mem_mtrace.o test_dma_mem_mops.o

TARGET = test_dma

all: $(TARGET)

# Rule to compile assembly files from parent directory into current directory
$(BDIR)/%.o: ../%.asm
	as $(ASMFLAGS) -o $@ $<

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

$(TARGET): $(OBJS)
	$(CXX) $(LDFLAGS) -o $@ $^

test: $(TARGET)
	./$(TARGET)

clean:
	rm -f *.o $(TARGET) $(OBJS_ASM)

.PHONY: all test clean
