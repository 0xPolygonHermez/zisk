#!/bin/bash

# sudo apt-get -y install libgtest-dev libomp-dev libgmp-dev libbenchmark-dev

touch CudaArch.mk

cd utils
make
cd ..
if ! [ -e utils/deviceQuery ]; then
    echo "Error buidling CUDA deviceQuery!"
    exit 1
fi

CAP=`./utils/deviceQuery | grep "CUDA Capability" | head -n 1 | tr -d ' ' | cut -d ':' -f 2 | tr -d '.'`
if [ -z "$CAP" ]; then
    echo "Unable to get CUDA capability on this system!"
    exit 1
fi

echo "CUDA_ARCH = sm_$CAP" > CudaArch.mk
