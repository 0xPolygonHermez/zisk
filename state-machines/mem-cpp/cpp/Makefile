# Makefile to build static library for Rust FFI

CXX := g++
CXXFLAGS := -O3 -std=c++17 -mavx2 -march=native -Wall -Wextra -pthread -Iinclude -no-pie -fPIC
# CXXFLAGS := -O3 -g -std=c++17 -mavx2 -march=native -Wall -Wextra -pthread
# valgrind flags
# CXXFLAGS := -O1 -g -std=c++17 -Wall -Wextra -pthread

TARGET := libmemcpp.a

SRCS := tools.cpp api.cpp mem_count_and_plan.cpp immutable_mem_planner.cpp \
		mem_align_counter.cpp mem_check_point.cpp mem_context.cpp mem_counter.cpp \
		mem_locators.cpp mem_segment_hash_table.cpp mem_planner.cpp

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
