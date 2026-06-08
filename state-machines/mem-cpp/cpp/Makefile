# Makefile to build the static C++ library for Rust FFI.


UNAME_S := $(shell uname -s)

ifeq ($(UNAME_S),Darwin)
    HOMEBREW_PREFIX := $(shell brew --prefix 2>/dev/null || echo /opt/homebrew)
    CXX      := clang++
    CXXFLAGS := -O3 -std=c++20 -Wall -Wextra -pthread -fPIC \
                -Xpreprocessor -fopenmp \
                -I$(HOMEBREW_PREFIX)/opt/libomp/include \
                -D__DARWIN_ARCH__ -Iinclude $(EXTRA_CXXFLAGS)
else
    CXX      := g++
    CXXFLAGS := -O3 -std=c++20 -Wall -Wextra -pthread -fopenmp \
                -no-pie -fPIC -Iinclude $(EXTRA_CXXFLAGS)
endif

TARGET := libmemcpp.a

SRCS := tools.cpp api.cpp mem_count_and_plan.cpp immutable_mem_planner.cpp \
		mem_align_counter.cpp mem_check_point.cpp mem_context.cpp mem_counter.cpp \
		mem_locators.cpp mem_segment_hash_table.cpp mem_planner.cpp mem_offsets.cpp

OBJS := $(addprefix $(OUT_DIR)/, $(SRCS:.cpp=.o))

# Find all header files to track dependencies
HEADERS := $(wildcard *.hpp)

all: $(OUT_DIR)/$(TARGET)

$(OUT_DIR)/$(TARGET): $(OBJS)
	ar rcs $@ $^

# Each object file depends on all header files to trigger rebuild when headers change
$(OUT_DIR)/%.o: %.cpp $(HEADERS)
	mkdir -p $(OUT_DIR)
	$(CXX) $(CXXFLAGS) -c $< -o $@

clean:
	rm -f *.o build/$(TARGET)

.PHONY: all clean
