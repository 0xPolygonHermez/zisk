# Makefile for encode_memcpy_inline test

CXX = g++
CXXFLAGS = -std=c++17 -O0 -Wall -Wextra -g -no-pie
# CXXFLAGS = -std=c++17 -O2 -Wall -Wextra -g -no-pie
LDFLAGS = -no-pie
ASMFLAGS = -g --noexecstack -I$(CURDIR)

SRCS_ASM = direct_memcpy_mtrace.asm memcpy_fast.asm fast_dma_encode.asm direct_memcpy_mops.asm
SRCS_CPP = test_dma.cpp
OBJS_ASM = direct_memcpy_mtrace.o memcpy_fast.o fast_dma_encode.o direct_memcpy_mops.o
OBJS_CPP = $(SRCS_CPP:.cpp=.o)
OBJS = $(OBJS_ASM) $(OBJS_CPP)
TARGET = test_dma

all: $(TARGET)

%.o: %.asm
	as $(ASMFLAGS) -o $@ $<

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

$(TARGET): $(OBJS)
	$(CXX) $(LDFLAGS) -o $@ $^

test: $(TARGET)
	./$(TARGET)

clean:
	rm -f $(OBJS) $(TARGET)

.PHONY: all test_dma clean
