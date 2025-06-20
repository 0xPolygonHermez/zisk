# Makefile to build static library for Rust FFI

CXX := g++
CXXFLAGS := -O3 -std=c++17 -mavx2 -march=native -Wall -Wextra -pthread -Iinclude -no-pie -fPIC

TARGET := libmemcpp.a

SRCS := tools.cpp api.cpp mem_count_and_plan.cpp
OBJS := $(addprefix $(OUT_DIR)/, $(SRCS:.cpp=.o))

all: $(OUT_DIR)/$(TARGET)

$(OUT_DIR)/$(TARGET): $(OBJS)
	ar rcs $@ $^

$(OUT_DIR)/%.o: %.cpp
	mkdir -p $(OUT_DIR)
	$(CXX) $(CXXFLAGS) -c $< -o $@

clean:
	rm -f *.o build/$(TARGET)

.PHONY: all clean
