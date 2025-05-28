# Makefile to build static library for Rust FFI

CXX := g++
CXXFLAGS := -O3 -std=c++17 -mavx2 -march=native -Wall -Wextra -pthread -Iinclude

TARGET := libmemcpp.a

SRCS := tools.cpp api.cpp
OBJS := $(SRCS:.cpp=.o)

all: $(OUT_DIR)/$(TARGET)

$(OUT_DIR)/$(TARGET): $(OBJS)
	ar rcs $@ $^

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

clean:
	rm -f *.o build/$(TARGET)

.PHONY: all clean
