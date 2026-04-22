GPU_ARCH := sm_120


count_and_plan: count_and_plan.cu
	nvcc -O3 -arch=$(GPU_ARCH) -std=c++20 -Xcompiler -fopenmp -lgomp -o $@ $<

clean:
	rm -f cout_and_plan

.PHONY: clean
