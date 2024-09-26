# Goldilocks

## Setup
### Dependencies
```
$ sudo apt-get install libgtest-dev
```

## Usage
Compile:
```
g++ tests/tests.cpp src/* -lgtest -lgmp -lomp -o test -g  -Wall -pthread -fopenmp -mavx2 -L$(find /usr/lib/llvm-* -name "libomp.so" | sed 's/libomp.so//')
```
```
g++ benchs/bench.cpp src/* -lbenchmark -lomp -lpthread -lgmp  -std=c++17 -Wall -pthread -fopenmp -mavx2 -L$(find /usr/lib/llvm-* -name "libomp.so" | sed 's/libomp.so//') -O3 -o bench
```
Example:
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

