NVCC      ?= nvcc
GPU_ARCH  ?= sm_80
NVCCFLAGS := -O3 -std=c++20 -arch=$(GPU_ARCH) -Xcompiler -fPIC

TARGET := libmemcpp_cu.a
SRCS   := count_and_plan.cu count_and_plan_c.cu
OBJS   := $(addprefix $(OUT_DIR)/, $(SRCS:.cu=.o))

all: $(OUT_DIR)/$(TARGET)

$(OUT_DIR)/$(TARGET): $(OBJS)
	ar rcs $@ $^

$(OUT_DIR)/%.o: %.cu
	mkdir -p $(OUT_DIR)
	$(NVCC) $(NVCCFLAGS) -c $< -o $@

clean:
	rm -f $(OUT_DIR)/*.o $(OUT_DIR)/$(TARGET)

.PHONY: all clean
