# Goldilocks

## Setup
### Dependencies
```
sudo apt-get install libgtest-dev libomp-dev libgmp-dev libbenchmark-dev
```

For CUDA 12.3:

```
sudo apt install nvidia-driver-535
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update
sudo apt-get -y install cuda-toolkit-12-3
```

## Usage
Compile:
```
g++ tests/tests.cpp src/*.cpp -lgtest -lgmp -lomp -o test -g  -Wall -pthread -fopenmp -mavx2 -L$(find /usr/lib/llvm-* -name "libomp.so" | sed 's/libomp.so//')
```
```
g++ benchs/bench.cpp src/*.cpp -lbenchmark -lomp -lpthread -lgmp  -std=c++17 -Wall -pthread -fopenmp -mavx2 -L$(find /usr/lib/llvm-* -name "libomp.so" | sed 's/libomp.so//') -O3 -o bench
```

## CUDA support

CUDA code is enabled with the flag ``__USE_CUDA__``.

To build the tests running only on CPU: ``make testscpu``.

To build the tests running on CPU+GPU: ``make testsgpu``.

Similarly for benchmarks: ``make benchcpu`` and ``make benchgpu``.

The script [``configure.sh``](configure.sh) finds the CUDA capability of the GPU installed on the host system and updates the file [``CudaArch.mk``](CudaArch.mk) which is used by the Makefile.

## Profiling and Timers

Timers (similar to those in zkProver) can be enabled with the flag ``GPU_TIMING``.

## Original Readme - Example of Usage
```cpp
#include <iostream>

#include "src/goldilocks_base_field.hpp"

int main(int argc, char **argv)
{

    Goldilocks::Element a = Goldilocks::fromU64(0xFFFFFFFF00000005ULL);
    uint64_t b = Goldilocks::toU64(a);
    Goldilocks::Element c = Goldilocks::fromString("6277101731002175852863927769280199145829365870197997568000");

    std::cout << Goldilocks::toString(a) << " " << b << " " << Goldilocks::toString(c) << "\n";

    return 0;
}
```
